/*
* This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
* This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
* You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 
*/

use std::{env, fs};
use std::sync::mpsc;
use std::thread;
use std::io::prelude::*;
use std::io::stdout;
use steel::{steel_vm::engine::Engine, SteelVal};
use steel::steel_vm::register_fn::RegisterFn;
use steel_derive::Steel;
use std::time::Duration;
use rumqttc::{MqttOptions,  Client, Connection, Event, Packet};
use chrono::{DateTime, Local};
use std::collections::HashMap;

mod utils; 

const PROGRAM_NAME: &'static str = "heinzelmann";

#[derive(Clone, Debug, Steel, PartialEq)]
struct TimedEvent {
    time: (u32, u32),
    id: String,
}

impl TimedEvent {
    fn new(time: (u32, u32), id: String) -> Self {
        return Self { time, id, };
    }
    fn register(time: String, topic: String) -> Self {
        let split: Vec<&str> = time.split(':').collect();
        let hours = split[0].parse().unwrap();
        let minutes = split[1].parse().unwrap();
        return Self::new((hours, minutes), topic);
    }
    fn get_next_time(&self) -> DateTime<Local> {
        let (hours, minutes) = self.time;
        let time = Local::now().date_naive().and_hms_opt(hours, minutes, 0).unwrap();
        let mut time = time.and_local_timezone(Local).unwrap();
        if Local::now().time() - time.time() >= chrono::Duration::minutes(0) {
            time = time + chrono::Duration::days(1);
        }
        return time;
    }
}

struct Configuration {
    id: String,
    program_location: String,
    addr: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
}

impl Configuration {
    fn new(id: String, program_location: String, addr: String, port: u16, user: Option<String>, password: Option<String>) -> Configuration {
        return Configuration { id, program_location, addr, port, user, password };
    }

    fn from_config_program(program: String) -> Configuration {
        let mut vm = Engine::new();
        vm.compile_and_run_raw_program(&program).unwrap();

        let id = match vm.extract_value("client_id") {
            Result::Ok(val) => val.try_into().unwrap(),
            Result::Err(_) => PROGRAM_NAME.into(),
        };

        let program_location = match vm.extract_value("program_location") {
            Result::Ok(val) => val.try_into().unwrap(),
            Result::Err(_) => format!("/etc/{}/program.scm", PROGRAM_NAME),
        };

        let addr = vm.extract_value("broker_addr").unwrap().try_into().unwrap();

        let port = match vm.extract_value("broker_port") {
            Result::Ok(val) => val.try_into().unwrap(),
            Result::Err(_) => 1883,
        };

        let user = match vm.extract_value("broker_user") {
            Result::Ok(val) => Some(val.try_into().unwrap()),
            Result::Err(_) => None,
        };
        let password = match vm.extract_value("broker_pass") {
            Result::Ok(val) => Some(val.try_into().unwrap()),
            Result::Err(_) => None,
        };

        return Configuration::new(id, program_location, addr, port, user, password);
    }

    fn connect(&self) -> (Client, Connection) {
        let mut mqttoptions: MqttOptions = MqttOptions::new(&self.id, &self.addr, self.port);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        if let Some(user) = &self.user {
            if let Some(password) = &self.password {
                mqttoptions.set_credentials(user, password);
            }
        }

        let (client, connection) = Client::new(mqttoptions, 10);
        return (client, connection);
    }

    fn get_program(&self) -> String {
        return get_file_contents(&self.program_location);
    }
}

fn get_file_contents(location: &str) -> String {
    let contents = fs::read_to_string(location)
        .expect(&format!("Unable to read file at {}.", location));
    return contents;
}


enum VMMessage {
    Command(ReplCommand),
    MqttConnect(Client),
    TimersReady(mpsc::Sender<TimedEvent>),
}

struct ReplCommand {
    cmd: String,
    response_tx: mpsc::Sender<ReplResponse>,
}

impl ReplCommand {
    fn new(cmd: String, response_tx: mpsc::Sender<ReplResponse>) -> ReplCommand {
        return ReplCommand { cmd, response_tx };
    }

    fn create(cmd: String) -> (ReplCommand, mpsc::Receiver<ReplResponse>) {
        let (resp_tx, resp_rx): (mpsc::Sender<ReplResponse>, mpsc::Receiver<ReplResponse>) = mpsc::channel();
        let repl_cmd = ReplCommand::new(cmd, resp_tx);
        return (repl_cmd, resp_rx);
    }
}

#[derive(Clone, Debug, Steel, PartialEq)]
enum HooksVariant {
    Simple,
    Tree,
}

#[derive(Clone, Debug, Steel, PartialEq)]
struct Hooks {
    variant: HooksVariant,
    hooks: HashMap<String, SteelVal>,
}

impl Hooks {
    fn new(variant: HooksVariant) -> Hooks {
        let hooks = HashMap::new();
        return Hooks { variant, hooks };
    }
    fn add_hook(&mut self, topic: SteelVal, f: SteelVal) -> SteelVal {
        if let SteelVal::StringV(s) = topic {
            self.hooks.insert(s.to_string(), f);
            return true.into();
        }
        else {
            return false.into();
        }
    }
    fn find_hook(&self, topic: SteelVal) -> SteelVal {
        if let SteelVal::StringV(s) = topic {
            let f = self.hooks.get(&s.to_string());
            match self.variant {
                HooksVariant::Simple => {
                    if let Some(f) = f {
                        return f.clone();
                    }
                },
                HooksVariant::Tree => {
                    let mut f = f;
                    let mut topic_parts: Vec<String> = s.split("/").map(|s| String::from(s.to_owned())).collect();
                    if s.starts_with("/") {
                        topic_parts.remove(0);
                        topic_parts[0] = "/".to_string() + &topic_parts[0];
                    }
                    while f == None && topic_parts.len() > 0 {
                        topic_parts = topic_parts[0..topic_parts.len()-1].to_vec();
                        let s = topic_parts.join("/") + "/#";
                        f = self.hooks.get(&s);
                    }
                    if let Some(f) = f {
                        return f.clone();
                    }
                    else if let Some(f) = self.hooks.get("#") {
                        return f.clone();
                    }
                },
            }
        }
        return false.into();
    }
}

fn timer_thread(repl_tx: mpsc::Sender<VMMessage>) {
    let timer_guy = timer::Timer::new();
    let mut guards = vec![];

    let (tx, rx): (mpsc::Sender<TimedEvent>, mpsc::Receiver<TimedEvent>) = mpsc::channel();
    repl_tx.send(VMMessage::TimersReady(tx)).unwrap();

    for inc in rx {
        let rtx = repl_tx.clone();
        let _guard = timer_guy.schedule(
                inc.get_next_time(), 
                Some(chrono::Duration::days(1)), 
                move || {
                    let cmd = format!(r#"(handle-timer "{}")"#, inc.id);
                    let (replcmd, rx) = ReplCommand::create(cmd);
                    rtx.send(VMMessage::Command(replcmd)).unwrap();
                    rx.recv().unwrap();
                });
        guards.push(_guard);
    }
}

fn vm_thread(rx: mpsc::Receiver<VMMessage>, program: String) {
    let mut vm = Engine::new();

    // REGISTERING BASIC UTILITY FUNCTIONS

    // Generates a random string of a specified length. Useful for message identifiers like those
    // that the Meross plugs use.
    vm.register_fn("random-string", utils::random_string);
    // Returns the current unix time stamp as an integer. This will be removed in favour of the
    // steel's integrated current-second function as soon as we upgrade steel.
    vm.register_fn("current-timestamp", utils::current_timestamp);
    // Returns the md5 hash of a list of strings as a string. Meross needs this, and maybe some
    // other systems as well.
    vm.register_fn("md5", utils::get_md5);

    // SETTING UP HOOKS IMPLEMENTATION
    
    let f: SteelVal = vm.run(r#"
        (lambda (topic msg)
          (displayln (string-append "Got message '" msg "' on topic '" topic "'.")))
        "#).unwrap().last().unwrap().clone();

    vm.register_type::<Hooks>("Hooks?");
    vm.register_fn("add-hook!", Hooks::add_hook);
    vm.register_fn("find-hook", Hooks::find_hook);

    // SETTING UP EVENT HOOKS
    let mut event_hooks = Hooks::new(HooksVariant::Tree);
    event_hooks.add_hook(SteelVal::StringV("#".into()), f);
    vm.register_external_value("event-hooks", event_hooks).unwrap();

    // SETTING UP TIMER HOOKS
    let timer_hooks = Hooks::new(HooksVariant::Simple);
    vm.register_external_value("timer-hooks", timer_hooks).unwrap();

    vm.run(r#"
            (define (handle-event topic msg) 
              ((find-hook event-hooks topic) topic msg))
            (define (handle-timer id)
              ((find-hook timer-hooks id)))

            (define (register-event! topic f) 
              (add-hook! event-hooks topic f))
            (define (register-timer! id f) 
              (add-hook! timer-hooks id f))
           "#).unwrap();

    // RUNNING PROGRAM
    let mut pre_flight_checks_mqtt = false;
    let mut pre_flight_checks_timers = false;
    let mut program_run = false;
    for inc in rx {
        match inc {
            VMMessage::Command(cmd) => {
                let result = vm.compile_and_run_raw_program(&cmd.cmd);
                match result {
                    Ok(r) => match r.last() {
                        Some(v) => match v {
                            SteelVal::Void => cmd.response_tx.send(ReplResponse::Empty).unwrap(),
                            SteelVal::StringV(s) => cmd.response_tx.send(ReplResponse::Return(s.to_string())).unwrap(),
                            _ => {
                                if let SteelVal::StringV(s) = vm.call_function_by_name_with_args("to-string", vec![v.to_owned()]).unwrap() {
                                    cmd.response_tx.send(ReplResponse::Return(s.to_string())).unwrap();
                                }
                            }
                        },
                        None => cmd.response_tx.send(ReplResponse::Empty).unwrap(),
                    },
                    Err(e) => {
                        vm.raise_error(e.clone());
                        cmd.response_tx.send(ReplResponse::Error(e.to_string())).unwrap();
                    },
                };
            },
            VMMessage::MqttConnect(c) => {
                vm.register_fn("send-simple", utils::send_closure(c.clone(), false));
                vm.register_fn("send-retain", utils::send_closure(c.clone(), true));
                vm.register_fn("subscribe", utils::subscribe_closure(c));
                pre_flight_checks_mqtt = true;
            },
            VMMessage::TimersReady(tx) => {
                vm.register_fn("set-timer", utils::set_timer_closure(tx));
                pre_flight_checks_timers = true;
            },
        }
        if !program_run && pre_flight_checks_mqtt && pre_flight_checks_timers {
            vm.compile_and_run_raw_program(&program).unwrap(); //TODO: Signify when the service fails because the provided program fails
            program_run = true;
        }
    }
}



enum ReplResponse {
    Empty,
    Return(String),
    Error(String),
}

fn repl_thread(tx: mpsc::Sender<VMMessage>) {
    let stdin = std::io::stdin();
    let mut buf = String::new();

    loop {
        print!(">>> ");
        stdout().flush().unwrap();
        stdin.read_line(&mut buf).unwrap();
        let tx_line = buf.clone()
            .strip_suffix("\n").unwrap()
            .to_string();
        if tx_line.starts_with("(quit)") {
            break;
        }
        let (repl_cmd, resp_rx) = ReplCommand::create(tx_line);
        tx.send(VMMessage::Command(repl_cmd)).unwrap();
        buf = "".into();
        match resp_rx.recv().unwrap() {
            ReplResponse::Empty => println!("=> ()"),
            ReplResponse::Return(s) => println!("=> {}", s),
            ReplResponse::Error(_) => {},
        }
    }
}

fn main() {
    println!("Starting {}...", PROGRAM_NAME);
    let args: Vec<String> = env::args().collect();
    let config_location: String = {
        if args.len() > 1 {
            args[1].clone()
        }
        else {
            format!("/etc/{}/config.scm", PROGRAM_NAME).into()
        }
    };
    let config_program = get_file_contents(&config_location);
    let config = Configuration::from_config_program(config_program);
    let program = config.get_program();

    let (tx, rx): (mpsc::Sender<VMMessage>, mpsc::Receiver<VMMessage>) = mpsc::channel();
    thread::spawn(move || vm_thread(rx, program));

    let repl_tx = tx.clone();
    thread::spawn(move || repl_thread(repl_tx));

    let timer_tx = tx.clone();
    thread::spawn(move || timer_thread(timer_tx));

    let (client, mut conn) = config.connect();
    tx.send(VMMessage::MqttConnect(client)).unwrap();

    for (_, notification) in conn.iter().enumerate() {
        let event = notification.unwrap();
        match event {
            Event::Incoming(packet) => match packet {
                Packet::Publish(inc) => {
                    let exp = format!(r#"(handle-event "{}" "{}")"#, inc.topic, str::replace(std::str::from_utf8(&inc.payload).unwrap(), "\"", "\\\""));
                    let (cmd, rx) = ReplCommand::create(exp.into());
                    tx.send(VMMessage::Command(cmd)).unwrap();
                    rx.recv().unwrap();
                },
                _ => (),
            },
            _ => (),
        }
    };
}
