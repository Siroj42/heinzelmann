# Heinzelmann

This little program helps you around the house by executing [MQTT](https://mqtt.org) automations defined in [Steel](https://github.com/mattwparas/steel), a dialect of [Scheme](https://www.scheme.org/) written in Rust.

## Usage

Before trying the example, you need to point heinzelmann at an MQTT broker of your choice. The relevant file is `example/config.scm`, where you can set your broker's URL and port as well as (optionally, otherwise, remove those lines) your login data.

Then, run `heinzelmann` via cargo and specify the config file:

```
cargo run -- example/config.scm
```

Or, build it with nix:

```
nix build
./result/bin/heinzelmann example/config.scm
```

The example program at `example/program.scm` can give you an idea of what `heinzelmann` is currently capable of.
