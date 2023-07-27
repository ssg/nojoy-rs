mod devenum;

use devenum::game_controllers;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Enable,
    Disable,
}

fn main() {
    let controllers = game_controllers().unwrap();
    for item in controllers {
        println!("{:?}", item);
    }
}
