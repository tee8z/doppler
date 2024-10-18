### How to use these scripts for local doppler nodes or remote nodes
- For local:
1. Run [local_setup.doppler](local_setup.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/local_setup.doppler"`
2. Open a new command console and run [setup_channels.doppler](setup_channels.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/setup_channels.doppler"`
3. In that same console, run [exchange_activity.doppler](exchange_activity.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/exchange_activity.doppler"`
4. Open another new command console and run [merchant_activity.doppler](merchant_activity.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/merchant_activity.doppler"`
- Leave these three command consoles running and you will see generated payment activity on your local cluster, this is handle for testing an application which uses lightning

- For Remote:
1. Setup your nodes somewhere on a remote server, right now doppler will only be able to support LND nodes that are remotely hosted but eventually it should be able to support Eclair and CoreLn as well. 
2. Once the LND nodes are provisioned and have some bitcoin on them, download the admin.macaroon and make note of the domain each is running on. You will need three remote nodes for this simulation
3. Follow how [external_nodes](../../external_nodes/info.example.conf) is setup, change the name of the file to `info.conf`. Place your admin macaroons in the respective folders under each alias for the nodes. Once the files are place in the location that info.conf expects for each node, move on to the next step
4. At this point you are running to actually run the doppler simulation against your nodes, if there aren't any channels setup with them yet run this script (at the root of the project):
[setup_channels.doppler](setup_channels.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/setup_channels.doppler" --external_nodes="external_nodes/info.conf"` 
5. In that same console, run [exchange_activity.doppler](exchange_activity.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/exchange_activity.doppler" --external_nodes="external_nodes/info.conf"`
6.  Open another new command console and run [merchant_activity.doppler](merchant_activity.doppler) via `cargo run doppler -- -f "doppler_files/external_nodes/merchant_activity.doppler" --external_nodes="external_nodes/info.conf"`
- Leave these two command consoles running and you will see generated payment activity on your remote cluster, this is handy for testing an application which use lightning and especially ones that need to validate against a large amount of historical lightning data.
