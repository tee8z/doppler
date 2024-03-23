use crate::{
    build_bitcoind, build_cln, build_eclair, build_lnd, build_visualizer,
    load_options_from_compose, run_cluster, DopplerParser, ImageInfo, L1Node, MinerTime,
    NodeCommand, NodeKind, Options, Rule,
};
use anyhow::{Error, Result};
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use slog::{debug, error, info};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, spawn},
    time::Duration,
    vec,
};
use uuid::Uuid;

const COMPOSE_PATH: &str = "doppler-cluster.yaml";

pub fn run_workflow(options: &mut Options, parsed: Pair<'_, Rule>) -> Result<(), Error> {
    for pair in parsed.into_inner() {
        match pair.as_rule() {
            Rule::loop_content => handle_loop(options, pair).expect("invalid loop block"),
            Rule::conf => handle_conf(options, pair).expect("invalid conf line"),
            Rule::up => handle_up(options).expect("failed to start the cluster"),
            Rule::skip_conf => {
                handle_skip_conf(options).expect("failed load current cluster into options")
            }
            Rule::ln_node_action => {
                handle_ln_action(options, pair).expect("invalid node action line")
            }
            Rule::btc_node_action => {
                handle_btc_action(options, pair).expect("invalid node action line")
            }
            Rule::EOI => {
                options
                    .clone()
                    .read_end_of_doppler_file
                    .as_ref()
                    .swap(true, Ordering::SeqCst);
                break;
            }
            _ => continue,
        }
    }
    Ok(())
}

pub fn run_workflow_until_stop(
    options: &mut Options,
    contents: std::string::String,
) -> Result<(), std::io::Error> {
    let parsed = DopplerParser::parse(Rule::page, &contents)
        .expect("parse error")
        .next()
        .unwrap();

    let main_thread_active = options.main_thread_active.clone();
    let all_threads = options.get_thread_handlers();
    run_workflow(options, parsed).unwrap();
    // if we have no child threads, this must be a script we just want to run through
    if all_threads.lock().unwrap().is_empty()
        && options.loop_count.as_ref().load(Ordering::SeqCst) == 0
    {
        main_thread_active.set(false);
        return Ok(());
    }
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&terminate))?;
    let mut current_loop_count = options.loop_count.as_ref().load(Ordering::SeqCst);
    let mut read_end_of_doppler_file = options
        .read_end_of_doppler_file
        .as_ref()
        .load(Ordering::SeqCst);
    while current_loop_count > 0 || !read_end_of_doppler_file {
        if terminate.load(Ordering::Relaxed) {
            break;
        }
        current_loop_count = options.clone().loop_count.as_ref().load(Ordering::SeqCst);
        read_end_of_doppler_file = options
            .read_end_of_doppler_file
            .as_ref()
            .load(Ordering::SeqCst);
        thread::sleep(Duration::from_secs(1));
    }
    main_thread_active.set(false);
    // wait for all child processes to be killed
    let mut handles = all_threads.lock().unwrap();
    // collect handles to release the lock
    let handles: Vec<_> = handles.drain(..).collect();
    // drop the collected handles to ensure they're joined
    drop(handles);
    Ok(())
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct LoopOptions {
    name: String,
    iterations: Option<i64>,
    sleep_time_interval_type: Option<char>,
    sleep_time_amt: Option<u64>,
}

impl Default for LoopOptions {
    fn default() -> Self {
        Self {
            name: String::from(""),
            iterations: None,
            sleep_time_interval_type: None,
            sleep_time_amt: None,
        }
    }
}

fn process_start_loop(line: Pair<Rule>) -> LoopOptions {
    let mut line_inner = line.into_inner();

    // move past loop command
    line_inner.next();
    let inner_pair = line_inner.next().unwrap();
    let mut loop_options = match inner_pair.as_rule() {
        Rule::num => process_loop_iter(line_inner, inner_pair),
        Rule::every => process_loop_sleep(line_inner, None),
        _ => unreachable!(),
    };

    let id = Uuid::new_v4();
    loop_options.name = id.to_string();
    loop_options
}

fn process_loop_iter(line_inner: Pairs<'_, Rule>, inner_pair: Pair<'_, Rule>) -> LoopOptions {
    let loop_options = if inner_pair.as_rule() == Rule::every {
        Some(LoopOptions::default())
    } else {
        Some(LoopOptions {
            iterations: Some(inner_pair.as_str().parse::<i64>().expect("invalid num")),
            ..Default::default()
        })
    };

    if line_inner.peek().is_some() {
        return process_loop_sleep(line_inner, loop_options);
    }

    loop_options.unwrap()
}

fn process_loop_sleep(
    mut line_inner: Pairs<'_, Rule>,
    mut loop_options: Option<LoopOptions>,
) -> LoopOptions {
    //move past every
    let peek_next = line_inner.peek();
    if peek_next.unwrap().as_rule() == Rule::every {
        line_inner.next();
    }
    if loop_options.is_none() {
        loop_options = Some(LoopOptions::default());
    }

    let (sleep_interval, sleep_time_type) = {
        let mut next = || line_inner.next().expect("invalid every command");
        (
            Some(next().as_str().parse::<u64>().expect("invalid num")),
            next().as_str().chars().next(),
        )
    };
    let mut raw_loop_options = loop_options.unwrap();
    raw_loop_options.sleep_time_amt = sleep_interval;
    raw_loop_options.sleep_time_interval_type = sleep_time_type;
    raw_loop_options
}

fn handle_loop(options: &mut Options, line: Pair<'_, Rule>) -> Result<()> {
    let line_inner = line.clone().into_inner();
    let logger = options.global_logger();

    let mut command_stack = vec![];
    let mut current_loop = None;
    for inner_pair in line_inner {
        match inner_pair.as_rule() {
            Rule::start => {
                debug!(logger, "processing start command");
                options.loop_count.as_ref().fetch_add(1, Ordering::SeqCst);
                current_loop = Some(process_start_loop(inner_pair));
            }
            Rule::btc_node_action => {
                debug!(logger, "processing btc node command");
                let node_command = process_btc_action(inner_pair);
                command_stack.push(node_command);
            }
            Rule::ln_node_action => {
                debug!(logger, "processing ln node command");
                let node_command = process_ln_action(inner_pair);
                command_stack.push(node_command);
            }
            Rule::end => {
                debug!(logger, "processing end command");
                run_loop(
                    options,
                    current_loop.clone().unwrap(),
                    command_stack.clone(),
                )?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

fn run_loop(
    options: &mut Options,
    loop_options: LoopOptions,
    loop_command_stack: Vec<NodeCommand>,
) -> Result<(), Error> {
    let current_options = options.clone();
    let thread_options = options.clone();
    debug!(
        options.global_logger(),
        "starting loop: {} command total: {}",
        loop_options.name,
        loop_command_stack.len()
    );
    spawn(move || {
        debug!(
            current_options.global_logger(),
            "in child thread for loop: {}", loop_options.name
        );
        let thread_handle = thread::current();
        thread_options.add_thread(thread_handle);
        let mut iter_count = -1;
        if loop_options.iterations.is_some() {
            iter_count = loop_options.iterations.unwrap();
        }
        debug!(
            current_options.global_logger(),
            "main thread active: {}",
            current_options.main_thread_active.val()
        );
        while current_options.main_thread_active.val() {
            if iter_count == 0 {
                debug!(
                    current_options.global_logger(),
                    "finished iterations, stopping loop: {}", loop_options.name
                );
                current_options
                    .loop_count
                    .as_ref()
                    .fetch_sub(1, Ordering::SeqCst);
                break;
            }
            if current_options.main_thread_paused.val() {
                debug!(
                    current_options.global_logger(),
                    "main thread paused, sleeping: {}", loop_options.name
                );
                thread::sleep(Duration::from_secs(1));
                continue;
            }
            for command in loop_command_stack.clone() {
                debug!(
                    current_options.global_logger(),
                    "running commands for loop: {}", loop_options.name
                );

                let action = match command.name.as_str() {
                    "MINE_BLOCKS" => node_mine_bitcoin(
                        &current_options.clone(),
                        command.to.to_owned(),
                        command.amt.unwrap(),
                    ),
                    "OPEN_CHANNEL" => open_channel(&current_options.clone(), &command),
                    "SEND_LN" => send_ln(&current_options.clone(), &command),
                    "SEND_ON_CHAIN" => send_on_chain(&current_options.clone(), &command),
                    "CLOSE_CHANNEL" => close_channel(&current_options.clone(), &command),
                    _ => unreachable!(),
                };
                match action {
                    Ok(_) => (),
                    Err(e) => error!(
                        current_options.global_logger(),
                        "error running an action in a loop: {}", e
                    ),
                };
            }
            if loop_options.sleep_time_amt.is_some() && loop_options.sleep_time_amt.is_some() {
                let sleep_time = match loop_options.sleep_time_interval_type.unwrap() {
                    's' => Duration::from_secs(loop_options.sleep_time_amt.unwrap()),
                    'm' => Duration::from_secs(loop_options.sleep_time_amt.unwrap() * 60),
                    'h' => Duration::from_secs(loop_options.sleep_time_amt.unwrap() * 60 * 60),
                    _ => unimplemented!(),
                };
                debug!(
                    current_options.global_logger(),
                    "pausing for specified amount of time loop: {}", loop_options.name
                );
                thread::sleep(sleep_time);
            }
            iter_count -= 1;
        }
    });
    Ok(())
}

fn get_image(options: &mut Options, node_kind: NodeKind, possible_name: &str) -> ImageInfo {
    let image_info = if !possible_name.is_empty() {
        if let Some(image) = options.get_image(possible_name) {
            image
        } else {
            options.get_default_image(node_kind)
        }
    } else {
        options.get_default_image(node_kind)
    };
    image_info
}

fn handle_conf(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let mut line_inner = line.into_inner();
    let command = line_inner.next().expect("invalid command");
    let mut inner = command.clone().into_inner();

    match command.clone().as_rule() {
        Rule::node_image => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let image_name = inner.next().expect("image name").as_str();
            let tag_or_path = inner.next().expect("image version").as_str();
            handle_image_command(options, kind, image_name, tag_or_path)?;
        }
        Rule::node_def => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            if kind == NodeKind::Visualizer {
                return Ok(());
            }
            let node_name = inner.next().expect("node name").as_str();
            let image = match inner.next() {
                Some(image) => get_image(options, kind.clone(), image.as_str()),
                None => options.get_default_image(kind.clone()),
            };
            handle_build_command(options, node_name, kind, &image, None)?;
        }
        Rule::node_miner => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let name = inner.next().expect("invalid image name").as_str();
            let image = get_image(options, kind.clone(), name);
            let time_num = inner
                .next()
                .expect("invalid time value")
                .as_str()
                .parse::<u64>()
                .expect("invalid time");
            let time_type = inner
                .next()
                .expect("invalid time type")
                .as_str()
                .chars()
                .next()
                .unwrap_or('\0');
            handle_build_command(
                options,
                name,
                kind,
                &image,
                BuildDetails::new_miner_time(MinerTime::new(time_num, time_type)),
            )?;
        }
        Rule::node_pair => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            let name = inner.next().expect("ident").as_str();
            let image = match inner.peek().unwrap().as_rule() {
                Rule::image_name => {
                    let image_name = inner.next().expect("image name").as_str();
                    get_image(options, kind.clone(), image_name)
                }
                _ => options.get_default_image(kind.clone()),
            };
            let to_pair = inner.next().expect("invalid layer 1 node name").as_str();
            let amount = match inner.peek().is_some() {
                true => inner
                    .next()
                    .expect("invalid amount")
                    .as_str()
                    .parse()
                    .unwrap(),
                false => 10000000,
            };
            handle_build_command(
                options,
                name,
                kind,
                &image,
                BuildDetails::new_pair(to_pair.to_owned(), amount),
            )?;
        }
        _ => (),
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct BuildDetails {
    pub pair: Option<NodePair>,
    pub miner_time: Option<MinerTime>,
}

#[derive(Debug, Default)]
pub struct NodePair {
    pub name: String,
    pub wallet_starting_balance: i64,
}

impl BuildDetails {
    pub fn new_pair(pair: String, amount: i64) -> Option<BuildDetails> {
        Some(BuildDetails {
            pair: Some(NodePair {
                name: pair,
                wallet_starting_balance: amount,
            }),
            miner_time: None,
        })
    }
    pub fn new_miner_time(miner_time: MinerTime) -> Option<BuildDetails> {
        Some(BuildDetails {
            pair: None,
            miner_time: Some(miner_time),
        })
    }
}

fn handle_image_command(
    option: &mut Options,
    kind: NodeKind,
    name: &str,
    tag_or_path: &str,
) -> Result<()> {
    let is_known_image = option.is_known_polar_image(kind.clone(), name, tag_or_path);

    let image = ImageInfo::new(
        tag_or_path.to_owned(),
        name.to_owned(),
        !is_known_image,
        kind,
    );
    option.images.push(image);
    Ok(())
}

fn handle_build_command(
    options: &mut Options,
    name: &str,
    kind: NodeKind,
    image: &ImageInfo,
    details: Option<BuildDetails>,
) -> Result<()> {
    match kind {
        NodeKind::Bitcoind => build_bitcoind(options, name, image, &None),
        NodeKind::BitcoindMiner => {
            build_bitcoind(options, name, image, &details.unwrap().miner_time)
        }
        NodeKind::Lnd => build_lnd(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Eclair => build_eclair(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Coreln => build_cln(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Visualizer => build_visualizer(options, name),
    }
}

fn handle_up(options: &mut Options) -> Result<(), Error> {
    run_cluster(options, COMPOSE_PATH).map_err(|e| {
        error!(
            options.global_logger(),
            "Failed to start cluster from generated compose file: {}", e
        );
        e
    })?;

    //pause until input
    info!(
        options.global_logger(),
        "doppler cluster has been created, please press enter to continue the script"
    );
    options.main_thread_paused.set(true);
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    options.main_thread_paused.set(false);
    debug!(options.global_logger(), "read in user input, continuing");
    Ok(())
}

fn handle_ln_action(options: &Options, line: Pair<Rule>) -> Result<()> {
    let command = process_ln_action(line);
    match command.name.as_str() {
        "OPEN_CHANNEL" => open_channel(options, &command),
        "SEND_LN" => send_ln(options, &command),
        "SEND_ON_CHAIN" => send_on_chain(options, &command),
        "CLOSE_CHANNEL" => close_channel(options, &command),
        "STOP_LN" => stop_l2_node(options, &command),
        "START_LN" => start_l2_node(options, &command),
        _ => {
            error!(
                options.global_logger(),
                "command not supported yet! {:?}", command.name
            );
            Ok(())
        }
    }
}

fn process_ln_action(line: Pair<Rule>) -> NodeCommand {
    let mut line_inner = line.into_inner();
    let (from_node, command_name, to_node, amt) = {
        let mut line_inner = line_inner.clone().peekable();
        let from_node = line_inner.next().expect("invalid input").as_str();
        let command_name = line_inner.next().expect("invalid input").as_str();
        if let None = line_inner.peek() {
            return NodeCommand {
                name: command_name.to_owned(),
                from: from_node.to_owned(),
                to: String::from(""),
                amt: None,
                subcommand: None,
            };
        }

        let to_node = line_inner.next().expect("invalid input").as_str();
        let amount_raw = line_inner.next().expect("invalid input").as_str();
        let mut amount = None;
        if !amount_raw.is_empty() {
            amount = Some(amount_raw.parse::<i64>().expect("invalid num"))
        }
        (from_node, command_name, to_node, amount)
    };

    let subcommand = line_inner.next().map(|pair| pair.to_string());

    NodeCommand {
        name: command_name.to_owned(),
        from: from_node.to_owned(),
        to: to_node.to_owned(),
        amt,
        subcommand,
    }
}

fn handle_btc_action(options: &Options, line: Pair<Rule>) -> Result<()> {
    let command = process_btc_action(line);
    match command.name.as_str() {
        "MINE_BLOCKS" => node_mine_bitcoin(options, command.to.to_owned(), command.amt.unwrap()),
        "STOP_BTC" => stop_l1_node(options, &command),
        "START_BTC" => start_l1_node(options, &command),
        _ => {
            error!(
                options.global_logger(),
                "command not supported yet! {:?}", command.name
            );
            Ok(())
        }
    }
}

fn process_btc_action(line: Pair<Rule>) -> NodeCommand {
    let mut line_inner = line.into_inner();
    let (btc_node, command_name, number) = {
        let mut line_inner = line_inner.clone().peekable();
        let btc_node = line_inner.next().expect("invalid input").as_str();
        let command_name = line_inner.next().expect("invalid input").as_str();
        if let None = line_inner.peek() {
            return NodeCommand {
                name: command_name.to_owned(),
                from: btc_node.to_owned(),
                to: String::from(""),
                amt: None,
                subcommand: None,
            };
        }

        let number = line_inner.next().expect("invliad input").as_str().parse::<i64>().expect("invalid num");
        (
            btc_node,
            command_name,
            number,
        )
    };

    //TODO: add bitcoind commands that need subcommand options
    let subcommand = line_inner.next().map(|pair| pair.to_string());
    NodeCommand {
        name: command_name.to_owned(),
        from: "".to_owned(),
        to: btc_node.to_owned(),
        amt: Some(number),
        subcommand,
    }
}

fn handle_skip_conf(options: &mut Options) -> Result<(), Error> {
    load_options_from_compose(options, COMPOSE_PATH)?;
    info!(
        options.global_logger(),
        "doppler cluster has been found and loaded, continuing with script"
    );
    Ok(())
}

fn node_mine_bitcoin(options: &Options, miner_name: String, amt: i64) -> Result<(), Error> {
    let bitcoind = options.get_bitcoind_by_name(miner_name.as_str())?;
    let _ = bitcoind.mine_bitcoin(options, amt);
    Ok(())
}

fn stop_l1_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let bitcoind = options.get_bitcoind_by_name(&node_command.from)?;
    let _ = bitcoind.stop(options);
    Ok(())
}

fn start_l1_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let bitcoind = options.get_bitcoind_by_name(&node_command.from)?;
    let _ = bitcoind.start(options);
    Ok(())
}

fn open_channel(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(node_command.from.as_str())?;
    from.open_channel(option, node_command)
}

fn send_ln(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(node_command.from.as_str())?;
    from.send_ln(option, node_command)
}

fn send_on_chain(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(node_command.from.as_str())?;
    from.send_on_chain(option, node_command)
}

fn close_channel(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(node_command.from.as_str())?;
    from.close_channel(option, node_command)
}

fn stop_l2_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    ln_node.stop(options)
}

fn start_l2_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    ln_node.start(options)
}
