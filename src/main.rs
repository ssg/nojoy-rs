mod devenum;

fn main() {
    let controllers = devenum::game_controllers().unwrap();
    for item in controllers {
        println!("{:?}", item);
    }
}
