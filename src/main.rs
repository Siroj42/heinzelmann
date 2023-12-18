/*
* This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
* This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
* You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 
*/

use std::{env, fs};
use std::ops::Deref;
use std::sync::mpsc;
use std::thread;
use steel::{steel_vm::engine::Engine, SteelVal};
use steel::steel_vm::register_fn::RegisterFn;
use steel_derive::Steel;
use std::time::Duration;
use rumqttc::{MqttOptions,  Client, Connection, Event, QoS, Packet};
use chrono::{DateTime, Local};
use bytes::Bytes;

const PROGRAM_NAME: &'static str = "heinzelmann";

struct IncomingMessage {
    topic: String,
    content: String,
}

impl IncomingMessage {
    fn new(topic: String, content: String) -> Self {
        return Self { topic, content, };
    }
}

struct OutgoingMessage {
    topic: String,
    text: String,
}

impl OutgoingMessage {
    fn new(topic: String, text: String) -> Self {
        return Self { topic, text, };
    }
}

#[derive(Clone, Debug, Steel, PartialEq)]
struct TimedEvent {
    time: (u32, u32),
    topic: String,
}

impl TimedEvent {
    fn new(time: (u32, u32), topic: String) -> Self {
        return Self { time, topic, };
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

    fn from_config_program(program: String) -> Configuration{
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

/*
fn make_set_hs100_closure(tx: mpsc::Sender<OutgoingMessage>) -> impl Fn(u8, bool) -> () {
    return move |number, state| {
        let state = match state {
            true => "on",
            false => "off",
        };
        let payload = format!("wifiplug_{}_{}", number, state);
        let outmsg: OutgoingMessage = OutgoingMessage::new("hs100".into(), payload); 
        tx.send(outmsg).unwrap();
    };
}*/

fn make_send_simple_closure(tx: mpsc::Sender<OutgoingMessage>) -> impl Fn(String, String) -> () {
    return move |topic, payload| {
        let outmsg: OutgoingMessage = OutgoingMessage::new(topic, payload); 
        tx.send(outmsg).unwrap();
    };
}

fn encode_payload(text: String) -> Bytes {
    return Bytes::from(text);
}

fn process_event(event: Event, tx: mpsc::Sender<IncomingMessage>) {
    let packet = match event {
        Event::Incoming(packet) => packet,
        _ => return,
    };
    let publish = match packet {
        Packet::Publish(publish) => publish,
        _ => return,
    };
    let topic: String = publish.topic;
    let content: String = std::str::from_utf8(&publish.payload).unwrap().to_string();
    tx.send(IncomingMessage::new(topic, content)).unwrap();
}

fn init_vm(otx: Option<mpsc::Sender<OutgoingMessage>>) -> Engine {
    let mut vm = Engine::new();
    vm.register_type::<TimedEvent>("TimedEvent?");
    vm.register_fn("TimedEvent", TimedEvent::register);
    if let Some(otx) = otx {
//        vm.register_fn("set-light", make_set_hs100_closure(otx.clone()));
        vm.register_fn("send-simple", make_send_simple_closure(otx));
    }
    else {
//        vm.register_fn("set-light", |_: bool| ());
        vm.register_fn("send-simple", |_: String, _: String| ());
    }
    return vm;
}

fn vm_spawner_thread(irx: mpsc::Receiver<IncomingMessage>, otx: mpsc::Sender<OutgoingMessage>, program: &str) {
    for inc in irx {
        let otx = otx.clone();
        let program = program.to_owned();
        thread::spawn( move || {
            let mut vm = init_vm(Some(otx));
            vm.compile_and_run_raw_program(&program).unwrap();
            let func = vm.extract_value(&inc.topic).unwrap();
            let content = SteelVal::StringV(inc.content.into());
            vm.call_function_with_args(func, vec![content]).unwrap();
        });
    }
}

fn mqtt_client_thread(client: Client, rx: mpsc::Receiver<OutgoingMessage>) {
    let mut client = client.clone();
    for inc in rx {
        thread::sleep(Duration::from_secs(1));
        let payload = encode_payload(inc.text);
        client.publish(inc.topic, QoS::AtLeastOnce, false, payload).unwrap();
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

    let (itx, irx): (mpsc::Sender<IncomingMessage>, mpsc::Receiver<IncomingMessage>) = mpsc::channel();
    let (otx, orx): (mpsc::Sender<OutgoingMessage>, mpsc::Receiver<OutgoingMessage>) = mpsc::channel();
    let (mut client, mut connection) = config.connect();

    let mut vm = init_vm(None);
    vm.compile_and_run_raw_program(&program).unwrap();
    let topics = vm.extract_value("topics").unwrap();
    if let SteelVal::ListV(topics) =topics {
        for topic in topics {
            if let SteelVal::StringV(topic) = topic {
                client.subscribe(topic.to_string(), QoS::AtMostOnce).unwrap();
            }
        }
    }

    let mut my_timers: Vec<TimedEvent> = vec![];
    let timers = vm.extract_value("timers").unwrap();
    if let SteelVal::ListV(timers) = timers {
        for timer in timers {
            if let SteelVal::Custom(timer) = timer {
                let t = &*timer.borrow();
                let timer = t.deref().as_any_ref().downcast_ref::<TimedEvent>().unwrap();
                my_timers.push(timer.clone());
            }
        }
    }

    let timer_guy = timer::Timer::new();
    let mut guards = vec![];
    for t in my_timers {
        let tx = itx.clone();
        let _guard = timer_guy.schedule(t.get_next_time(), Some(chrono::Duration::days(1)), move || {
            let _ignored = tx.send(IncomingMessage::new(t.topic.clone(), "".into()));
        });
        guards.push(_guard);
    };

    thread::spawn(move || vm_spawner_thread(irx, otx, &program));
    thread::spawn(move || mqtt_client_thread(client, orx));

    println!("Started {}, listening for MQTT events...", PROGRAM_NAME);
    
    for (_, notification) in connection.iter().enumerate() {
        process_event(notification.unwrap(), itx.clone());
    };
}
