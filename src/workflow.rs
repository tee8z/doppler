use crate::{
    build_bitcoind, build_cln, build_eclair, build_lnd, load_options_from_compose,
    load_options_from_external_nodes, run_cluster, DopplerParser, ImageInfo, L1Node, MinerTime,
    NodeCommand, NodeKind, Options, Rule, Tag,
};
use anyhow::{Error, Result};
use log::{debug, error, info};
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
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

    let mut command_stack = vec![];
    let mut current_loop = None;
    for inner_pair in line_inner {
        match inner_pair.as_rule() {
            Rule::start => {
                debug!("processing start command");
                options.loop_count.as_ref().fetch_add(1, Ordering::SeqCst);
                current_loop = Some(process_start_loop(inner_pair));
            }
            Rule::btc_node_action => {
                debug!("processing btc node command");
                let node_command = process_btc_action(inner_pair);
                command_stack.push(node_command);
            }
            Rule::ln_node_action => {
                debug!("processing ln node command");
                let node_command = process_ln_action(inner_pair);
                command_stack.push(node_command);
            }
            Rule::end => {
                debug!("processing end command");
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
        "starting loop: {} command total: {}",
        loop_options.name,
        loop_command_stack.len()
    );
    spawn(move || {
        debug!("in child thread for loop: {}", loop_options.name);
        let thread_handle = thread::current();
        thread_options.add_thread(thread_handle);
        let mut iter_count = -1;
        if loop_options.iterations.is_some() {
            iter_count = loop_options.iterations.unwrap();
        }
        debug!(
            "main thread active: {}",
            current_options.main_thread_active.val()
        );
        while current_options.main_thread_active.val() {
            if iter_count == 0 {
                debug!("finished iterations, stopping loop: {}", loop_options.name);
                current_options
                    .loop_count
                    .as_ref()
                    .fetch_sub(1, Ordering::SeqCst);
                break;
            }
            if current_options.main_thread_paused.val() {
                debug!("main thread paused, sleeping: {}", loop_options.name);
                thread::sleep(Duration::from_secs(1));
                continue;
            }
            for command in loop_command_stack.clone() {
                debug!("running commands for loop: {}", loop_options.name);

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
                    "FORCE_CLOSE_CHANNEL" => {
                        force_close_channel(&current_options.clone(), &command)
                    }
                    "STOP_LN" => stop_l2_node(&current_options.clone(), &command),
                    "START_LN" => start_l2_node(&current_options.clone(), &command),
                    "SEND_HOLD_LN" => send_hold_invoice(&current_options.clone(), &command),
                    "SETTLE_HOLD_LN" => settle_hold_invoice(&current_options.clone(), &command),
                    "WAIT" => wait_number_of_blocks(&current_options.clone(), &command),
                    _ => unreachable!(),
                };
                match action {
                    Ok(_) => (),
                    Err(e) => error!("error running an action in a loop: {}", e),
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
                    "pausing for specified amount of time loop: {}",
                    loop_options.name
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
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
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
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
            let node_name = inner.next().expect("node name").as_str();
            let image: ImageInfo = match inner.next() {
                Some(image) => get_image(options, kind.clone(), image.as_str()),
                None => options.get_default_image(kind.clone()),
            };
            handle_build_command(options, node_name, kind, &image, None)?;
        }
        Rule::node_pair => {
            let kind: NodeKind = inner
                .next()
                .expect("node")
                .try_into()
                .expect("invalid node kind");
            if options.external_nodes.is_some() && kind != NodeKind::Lnd {
                unimplemented!("can only support LND nodes at the moment for remote nodes");
            }
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
                false => 100000000,
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
        NodeKind::Bitcoind => build_bitcoind(options, name, image, false),
        NodeKind::BitcoindMiner => build_bitcoind(options, name, image, true),
        NodeKind::Lnd => build_lnd(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Eclair => build_eclair(options, name, image, &details.unwrap().pair.unwrap()),
        NodeKind::Coreln => build_cln(options, name, image, &details.unwrap().pair.unwrap()),
    }
}

fn handle_up(options: &mut Options) -> Result<(), Error> {
    run_cluster(options, COMPOSE_PATH).map_err(|e| {
        error!("Failed to start cluster from generated compose file: {}", e);
        e
    })?;

    //pause until input
    info!("doppler cluster has been created, please press enter to continue the script");
    options.main_thread_paused.set(true);
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    options.main_thread_paused.set(false);
    debug!("read in user input, continuing");
    Ok(())
}

fn handle_ln_action(options: &mut Options, line: Pair<Rule>) -> Result<()> {
    let command = process_ln_action(line);
    match command.name.as_str() {
        "OPEN_CHANNEL" => open_channel(options, &command),
        "SEND_LN" => send_ln(options, &command),
        "SEND_ON_CHAIN" => send_on_chain(options, &command),
        "CLOSE_CHANNEL" => close_channel(options, &command),
        "FORCE_CLOSE_CHANNEL" => force_close_channel(options, &command),
        "STOP_LN" => stop_l2_node(options, &command),
        "START_LN" => start_l2_node(options, &command),
        "SEND_HOLD_LN" => send_hold_invoice(options, &command),
        "SETTLE_HOLD_LN" => settle_hold_invoice(options, &command),
        "WAIT" => wait_number_of_blocks(options, &command),
        _ => {
            error!("command not supported yet! {:?}", command.name);
            Ok(())
        }
    }
}

fn process_ln_action(line: Pair<Rule>) -> NodeCommand {
    let line_inner = line.into_inner();
    let mut node_command = NodeCommand {
        ..Default::default()
    };
    for pair in line_inner {
        match pair.as_rule() {
            Rule::tag => {
                let inner = pair.into_inner();
                node_command.tag = Some(inner.as_str().to_owned());
            }
            Rule::image_name => {
                if node_command.from.is_empty() {
                    node_command.from = pair.as_str().to_owned();
                } else {
                    node_command.to = pair.as_str().to_owned();
                }
            }
            Rule::ln_blocks => {
                let pair = pair.into_inner();
                node_command.amt = Some(pair.as_str().parse::<i64>().expect("invalid num"));
            }
            Rule::ln_node_action_type => {
                node_command.name = pair.as_str().to_owned();
            }
            Rule::ln_amount => {
                let pair = pair.into_inner();
                node_command.amt = Some(pair.as_str().parse::<i64>().expect("invalid num"));
            }
            Rule::ln_timeout => {
                let mut inner = pair.into_inner();
                let mut time_num = inner
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

                // convert to seconds
                match time_type {
                    'h' => time_num = time_num * 60 * 60,
                    'm' => time_num = time_num * 60,
                    _ => (),
                }
                node_command.timeout = Some(time_num)
            }
            Rule::sub_command => {
                node_command.subcommand = Some(pair.as_str().to_owned());
            }
            //Ignore any other rules found at this level
            _ => (),
        }
    }
    node_command
}

fn handle_btc_action(options: &Options, line: Pair<Rule>) -> Result<()> {
    let command = process_btc_action(line);
    match command.name.as_str() {
        "MINE_BLOCKS" => node_mine_bitcoin(options, command.to.to_owned(), command.amt.unwrap()),
        "STOP_BTC" => stop_l1_node(options, &command),
        "START_BTC" => start_l1_node(options, &command),
        "SEND_COINS" => send_to_l2(options, &command),
        _ => {
            error!("command not supported yet! {:?}", command.name);
            Ok(())
        }
    }
}

fn process_btc_action(line: Pair<Rule>) -> NodeCommand {
    let line_inner = line.into_inner();
    let mut line_inner = line_inner.clone().peekable();
    let btc_node = line_inner.next().expect("invalid input").as_str();
    let command_name = line_inner.next().expect("invalid input").as_str();
    if let None = line_inner.peek() {
        return NodeCommand {
            name: command_name.to_owned(),
            from: btc_node.to_owned(),
            to: String::from(""),
            ..Default::default()
        };
    }
    let val = line_inner.next().expect("invalid input");
    if val.as_rule() == Rule::image_name {
        let to = val.as_str();
        let number = line_inner
            .next()
            .expect("invalid input")
            .as_str()
            .parse::<i64>()
            .expect("invalid num");
        let subcommand = line_inner.next().map(|pair| pair.to_string());
        NodeCommand {
            name: command_name.to_owned(),
            from: btc_node.to_owned(),
            to: to.to_owned(),
            amt: Some(number),
            subcommand,
            ..Default::default()
        }
    } else {
        let number = val.as_str().parse::<i64>().expect("invalid num");
        let subcommand = line_inner.next().map(|pair| pair.to_string());
        NodeCommand {
            name: command_name.to_owned(),
            from: "".to_owned(),
            to: btc_node.to_owned(),
            amt: Some(number),
            subcommand,
            ..Default::default()
        }
    }
}

fn handle_skip_conf(options: &mut Options) -> Result<(), Error> {
    if let Some(external_nodes_path) = options.external_nodes_path.clone() {
        //TODO: add reading from external nodes config and build nodes from there
        load_options_from_external_nodes(options, &external_nodes_path)?;
        info!("external nodes have been found and loaded, continuing with script");
    } else {
        load_options_from_compose(options, COMPOSE_PATH)?;
        info!("doppler cluster has been found and loaded, continuing with script");
    }
    Ok(())
}

fn node_mine_bitcoin(options: &Options, miner_name: String, amt: i64) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let bitcoind = options.get_bitcoind_by_name(&miner_name)?;
    let _ = bitcoind.mine_bitcoin(options, amt);
    Ok(())
}

fn stop_l1_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let bitcoind = options.get_bitcoind_by_name(&node_command.from)?;
    let _ = bitcoind.stop(options);
    Ok(())
}

fn start_l1_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let bitcoind = options.get_bitcoind_by_name(&node_command.from)?;
    let _ = bitcoind.start(options);
    Ok(())
}
fn send_to_l2(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let bitcoind = options.get_bitcoind_by_name(&node_command.from)?;
    let _ = bitcoind.clone().send_to_l2(options, node_command);
    Ok(())
}

fn open_channel(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(&node_command.from)?;
    from.open_channel(option, node_command)
}

fn send_ln(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(&node_command.from)?;
    from.send_ln(option, node_command)
}

fn send_on_chain(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(&node_command.from)?;
    from.send_on_chain(option, node_command)
}

fn close_channel(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(&node_command.from)?;
    from.close_channel(option, node_command)
}

fn force_close_channel(option: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let from = option.get_l2_by_name(&node_command.from)?;
    from.force_close_channel(option, node_command)
}

fn stop_l2_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    ln_node.stop(options)
}

fn start_l2_node(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    if options.external_nodes.is_some() {
        unimplemented!("command can only be used in a local docker compose network");
    }
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    ln_node.start(options)
}

fn send_hold_invoice(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    // 1) get an rHash from node that will pay the hold invoice, that allows them to have the secret (preimage)
    // which can be handed to the other node to settle the hold invoice
    // 2) create a hold invoice that has the rhash provided so it doesn't generate a new preimage
    // 3) done, hold invoice has been created and is inflight
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    let ln_to_node = options.get_l2_by_name(&node_command.to)?;
    let rhash = ln_node.get_rhash(options)?;

    //This will only work with 2 LND node types at the moment
    let payment_request = ln_to_node.create_hold_invoice(options, node_command, rhash.clone())?;
    options.save_tag(&Tag {
        name: node_command.tag.clone().unwrap(),
        val: rhash,
    })?;
    ln_node.pay_invoice(options, node_command, payment_request)
}

fn settle_hold_invoice(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    let ln_to_node = options.get_l2_by_name(&node_command.to)?;
    let tag_name = node_command.tag.clone().unwrap();
    let tag = options.get_tag_by_name(tag_name);
    let preimage = ln_to_node.get_preimage(options, tag.val.clone())?;
    //This will only work with 2 LND node types at the moment
    ln_node.settle_hold_invoice(options, preimage)
}

fn wait_number_of_blocks(options: &Options, node_command: &NodeCommand) -> Result<(), Error> {
    let ln_node = options.get_l2_by_name(&node_command.from)?;
    let num_of_blocks = node_command.amt.unwrap();
    ln_node.wait_for_block(options, num_of_blocks)
}
