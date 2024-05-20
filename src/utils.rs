/*
* This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
* This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.
* You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 
*/
use std::sync::mpsc;
use crate::TimedEvent;
use rumqttc::{Client, QoS};
use bytes::Bytes;
use rand::{distributions::Alphanumeric, Rng};
use chrono::Local;

pub fn subscribe_closure(client: Client) -> impl Fn(String) -> () {
    return move |topic| {
        let mut client = client.clone();
        client.subscribe(topic, QoS::AtMostOnce).unwrap();
    };
}

pub fn send_closure(client: Client, retain: bool) -> impl Fn(String, String) -> () {
    return move |topic, payload| {
        let mut client = client.clone();
        let payload = Bytes::from(payload);
        client.publish(topic, QoS::AtLeastOnce, retain, payload).unwrap();
    };
}

pub fn set_timer_closure(tx: mpsc::Sender<TimedEvent>) -> impl Fn(String, String) -> () {
    return move |time_str, id| {
        tx.send(TimedEvent::register(time_str, id)).unwrap();
    }
}

pub fn random_string(length: usize) -> String {
    let s: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect();
    return s.to_lowercase();
}

pub fn current_timestamp() -> i64 {
    return Local::now().timestamp();
}

pub fn get_md5(inputs: Vec<String>) -> String {
    let mut context = md5::Context::new();
    for input in inputs {
        context.consume(input);
    }
    let digest: [u8; 16] = context.compute().into();
    let hexdigest = hex::encode(digest);
    return hexdigest;
}
