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

Run `fuga --help` to list available commands. When you start the application, you'll see available shortcuts right there in the UI.

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
