# Heinzelmann

This little program helps you around the house by executing [MQTT](https://mqtt.org) automations defined in [Steel](https://github.com/mattwparas/steel), a dialect of [Scheme](https://www.scheme.org/) written in Rust. You can set automations to trigger on MQTT Messages or at specific times of day. In true Scheme fashion, you can directly interact with your automation script using a REPL on the local CLI, or a compatible [nREPL](https://nrepl.org) client over the network.

## Usage

Before trying the example, you need to point heinzelmann at an MQTT broker of your choice. The relevant file is `example/config.scm`, where you can set your broker's URL and port as well as (optionally, otherwise, remove those lines) your login data. They can also be used to enable or disable the local REPL and add add IPs to the nREPL whitelist so you can connect to a REPL remotely (this is currently only tested with the [shevek](https://git.sr.ht/~technomancy/shevek/) client.

Then, run `heinzelmann` via cargo and specify the config file:

```
cargo run -- example/config.scm
```

The example programs at `example/hs100.scm` and `example/meross.scm` can give you an idea of what `heinzelmann` is currently capable of.
