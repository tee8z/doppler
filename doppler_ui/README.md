# Doppler Visualizer and Script building helper
- This UI provided tools to make it easy to view what the cluster looks like and how to build a script that can create or perform actions on the cluster of nodes.
- Run released UI
```
  cd /.doppler/<release tag> && node ./build
```

- Run from "./doppler/doppler_ui" in the git repo (not recommended for non-developers):
```
UI_CONFIG_PATH=./ui_config npm run dev
```
- make sure to update `./ui_config/server.conf` to have correct path to doppler binary and full path to working directory
```
dopplerBinaryPath = /$HOME/doppler/target/distrib/doppler-x86_64-unknown-linux-gnu/doppler
currentWorkingDirectory = /$HOME/doppler/doppler_ui
```
- NOTE: you may need to copy the ./doppler/config into ./doppler_ui/, that's how to handle the following error if seen in the doppler logs from the UI:
```
thread 'main' panicked at doppler/src/bitcoind.rs:116:61:

called `Result::unwrap()` on an `Err` value: No such file or directory (os error 2)

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```
