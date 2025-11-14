# Fuga

🎵 Fuga is a tool for launching, monitoring, and managing multi-process applications. Think of it as a simpler version of docker-compose, without docker. Fuga was created in the context of writing software for robotics.
- Robotics software requires many systems to run in parallel ([Concurrency versus parallellism](https://stackoverflow.com/a/1050257)). For example, your camera might get frames at 15 Hz and your motion planner should compute trajectories as fast as possible, but your control loop must execute at 100 Hz without being delayed by the camera or motion planner.
- Some of these processes are slow to start (e.g., connecting to a camera or a robot can take several seconds.) Fuga supports rapid iteration during development by allowing you to keep certain processes running and only restart your main control loop.
- Writing robotics software is already complex. Tooling should make your life easier, not harder.

You supply your application requirements as a [TOML](https://toml.io/en/) file, and Fuga opens a [TUI](https://en.wikipedia.org/wiki/Text-based_user_interface) that allows you to monitor and control processes. This includes:
- Viewing logs
- Stopping processes
- Restarting processes

This makes the development of robotics applications smoother: you can hot-reload processes or stop problematic ones without ever leaving the terminal.

Once your application is ready for deployment, Fuga also has a daemon mode (Note: this is not yet implemented, but it is on the roadmap). In the daemon mode, Fuga will launch your application without starting the interactive UI. Processes that crash can automatically be restarted if so desired, boosting the reliability of your robot.

For example, you could launch an application with a camera process and a control loop, and restart the control loop after you've altered some logic. It's as simple as this:
```sh
fuga MyRobotApp.toml
```

TODO Screenshot goes here.

Fuga is work in progress software. Use it at your own risk: we're not responsible for messing up your system or hardware-related damage. The code is fully open-source and we're open to pull requests and audits of the code.

## Installation

For now, you'll need to build from source:

```sh
cargo build --release
```

This requires that [Rust and Cargo are installed](https://rustup.rs/).

## Usage

Run `fuga --help` for available arguments. When you start an application, you'll see available commands right there in the UI.

### Controlling the TUI

Fuga's UI has two tabs. On the left, you can see the process table. It contains all of the processes of your application, their process IDs, and their current status.
You can scroll through these using the arrow keys (up/down) or j and k.

On the right, there's the log tab. If you press the l (lowercase L) key, you'll show the logs for the currently highlighted process in this tab.
By pressing n, you can swap to the logs tab to scroll through these logs. Press n to return focus to the process tab.

You can (re)start a process using the r key, and stop it using the s key. Quit the application - killing all child processes - using q.

Fuga can also redirect logs to files, which may be useful if you want to monitor all processes at once, with `tail -f $LOGFILE`.

### Application files

The application file format is based on TOML. Here's an example showing all available fields:

```toml
name = "MyApp"

[env]
RERUN_RECORDING_ID="ab0a9772-cf9e-466b-a3e5-fd638d2e0ce2"

[[processes]]
name = "Rerun"
command = "uv"
args = ["run", "rerun", "--hide-welcome-screen"]
work_dir = "camera"
restart = "never"

[[processes]]
name = "Camera"
command = "uv"
args = ["run", "python", "camera.py"]
work_dir = "camera"
restart = "on-failure"
restart_delay_secs = 1.0

[[processes]]
name = "Ping"
command = "ping"
args = ["google.com"]
restart = "always"
```

You can set environment variables under the `[env]` header.

You can add individual processes under the `[[processes]]` headers. You can choose to automatically restart processes (after an optional delay) - the default is "never", set the working directory in which the programs are to be launched, and provide an alias (name) for easy reference to the process in Fuga.

## Why not...

### ROS/roslaunch?

We aim for a simple, non-viral development set-up. Your "nodes" don't need to know about Fuga. You can use any inter-process communication framework you want.
There's no need for building packages with a build tool like colcon, and you can use any programming language for any node, as you see fit.

✊ This lowers the barrier to entry for robotics software and gives freedom to the developer to use the technologies that fits their needs.

### Supervisord?

Supervisord is great, too complex for our use cases. For many cases, you don't need the amount of configuration it allows. Additionally, Fuga only has a single binary and allows for interactive control, improving the user experience.

## What's a fuga?

Fuga (pronounced something like *fooga*) is Dutch for [fugue](https://en.wikipedia.org/wiki/Fugue), a type of polyphonic musical composition (i.e., with multiple voices being interwoven).

## Citation

If you use Fuga in the context of academic research, please consider citing it. See `CITATION.cff`.

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
