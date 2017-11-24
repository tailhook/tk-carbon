extern crate abstract_ns;
extern crate futures;
extern crate futures_cpupool;
extern crate ns_router;
extern crate ns_std_threaded;
extern crate regex;
extern crate tk_carbon;
extern crate tk_easyloop;
extern crate tokio_core;
extern crate env_logger;

use std::env::args;
use std::io::{self, BufRead};
use std::thread;
use std::time::Duration;

use abstract_ns::{HostResolve};
use ns_router::{Router, SubscribeExt, Config as NsConfig};
use regex::Regex;
use tk_carbon::{Carbon, Config};
use tk_easyloop::handle;


fn main() {
    env_logger::init().expect("init logging");
    let name = args().skip(1).next().unwrap_or("localhost:2003".to_string());
    let (carbon, init) = Carbon::new(&Config::new().done());
    // run io in thread because stdin is not officially supported in
    // mio/tokio yet
    thread::spawn(|| {
        let mut keep_router = None;
        tk_easyloop::run_forever(|| {
            let router = Router::from_config(&NsConfig::new()
                .set_fallthrough(ns_std_threaded::ThreadedResolver::new()
                    .null_service_resolver()
                    .interval_subscriber(Duration::new(1, 0), &handle()))
                .done(), &tk_easyloop::handle());
            keep_router = Some(router.clone());
            init.connect_to(router.subscribe_many(&[name], 2003),
                &tk_easyloop::handle());
            Ok::<(), ()>(())
        }).unwrap();
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
            carbon.add_value(format_args!("test.{}", name), value);
            println!("Metric {:?} value {:?}",
                format_args!("test.{}", name),
                value);
        } else {
            println!("Invalid format. Use `metric.name 1235`.");
        }
    }
}
