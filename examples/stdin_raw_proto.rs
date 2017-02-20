extern crate futures;
extern crate regex;
extern crate tk_easyloop;
extern crate tk_carbon;

use std::io::{self, BufRead};
use std::thread;

use futures::sync::mpsc::unbounded;
use regex::Regex;
use tk_carbon::Proto;
use tokio_core::net::TcpStream;


fn main() {
    let (tx, rx) = unbounded();
    // run io in thread because stdin is not officially supported in
    // mio/tokio yet
    thread::spawn(|| {
        tk_easyloop::run(|| {
            Ok(());
        })
    });
    let regex = Regex::new(r"^([a-zA-Z0-9\.-]+)(?:\s+(\d+))$").unwrap();
    println!("Enter `metric.name 134`:");

    for line in io::BufReader::new(io::stdin()).lines() {
        let line = match line {
            Ok(x) => x,
            Err(_) => return,
        };
        if let Some(capt) = regex.captures(&line) {
            let name = capt.get(1).unwrap().as_str();
            let value: i64 = capt.get(2).unwrap().as_str().parse().unwrap();
            println!("Metric {:?} value {:?}", name, value);
        } else {
            println!("Invalid format. Use `metric.name 1235`.");
        }
    }
}
