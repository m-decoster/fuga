# Fuga

🎵 Fuga is a tool for launching, monitoring, and managing multi-process applications. Think of it as a simpler version of docker-compose, without the docker part. Fuga was created in the context of writing software for robotics.
- Robotics software requires many systems to run in parallel ([Concurrency versus parallellism](https://stackoverflow.com/a/1050257)). For example, your camera might get frames at 15 Hz and your motion planner should compute trajectories as fast as possible, but your control loop must execute at 100 Hz without being delayed by the camera or motion planner.
- Some of these processes are slow to start (e.g., connecting to a camera or a robot can take several seconds.) We support rapid iteration during development by allowing you to keep certain processes running and only restart your main control loop.
- Writing robotics software is already complex. Tooling should make your life easier, not harder.

For example, you could launch an application with a camera process and a control loop, and restart the control loop after you've altered some logic:
```sh
# ... assumes the application was launched already...
fugactl application MyRobotApp restart Logic
```

Fuga is work in progress software. Use it at your own risk: we're not responsible for messing up your system or hardware-related damage. The code is fully open-source and we're open to pull requests and audits of the code.

## Concepts

There are two applications that you should know about:
- `fugad` is a daemon that monitors running applications.
- `fugactl` is a CLI for launching and monitoring applications.

Fuga launches *applications*, which consist of one or more *processes*.
Processes can define a restarting policy, which determines if the process should restart, in which case (failure or always) and, optionally, after a delay.

When you run `fugactl start launch_file.toml`, `fugactl` will start all processes defined by the launch file and notify `fugad` of running processes by forwarding:
- The name of the application (defined in the launch file)
- The name and PID of each subprocess

Processes are launched as the user running `fugactl`.

## Installation

Note that Fuga requires [systemd](https://systemd.io/), which comes pre-installed on modern Ubuntu systems.

For now, you'll need to build from source:

```sh
cargo build --release
```

This requires that [Rust and Cargo are installed](https://rustup.rs/).

Then, run:

```sh
sudo ./installation/install_fugad.sh
```

to install `fugad` and `fugactl` under `/usr/local/bin`.

A service file (`./installation/packaging/fugad.service`) will also be installed.
The installation script will prompt you to start the daemon.

## Usage

Run `fugactl --help` to list available commands.

## Why not ROS/roslaunch?

We aim for a simple, non-viral development set-up. Your "nodes" don't need to know about Fuga. You can use any inter-process communication framework you want.
There's no need for building packages with a build tool like colcon, and you can use any programming language for any node, as you see fit.

✊ This lowers the barrier to entry for robotics software and gives freedom to the developer to use the technologies that fits their needs.

## What's a fuga?

Fuga (pronounced something like *fooga*) is Dutch for [fugue](https://en.wikipedia.org/wiki/Fugue), a type of polyphonic musical composition (i.e., with multiple voices being interwoven).

## Citation

If you use Fuga in the context of academic research, please consider citing us. See `CITATION.cff`.

```bibtex
@software{De_Coster_Fuga_an_orchestrator_2025,
    author = {De Coster, Mathieu},
    month = nov,
    title = {{Fuga: an orchestrator for multi-process robotics applications}},
    url = {https://github.com/m-decoster/fuga},
    version = {0.1.0},
    year = {2025}
}
```
