mod devenum;

use devenum::{disable_device, enable_device, game_controllers};
use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Enable,
    Disable,
}

#[derive(Debug, Clone, Subcommand)]
pub enum MainCommand {
    List,
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {

    #[command(subcommand)]
    pub command: MainCommand,
}

// example output:
// GameController { manufacturer: "(Standard system devices)", name: "HID-compliant game controller", instance_id: "HID\\{00001124-0000-1000-8000-00805F9B34FB}&VID_045E&PID_02E0&IG_00\\D&5688A0B&0&0000", status: Enabled, disableable: true }

fn main() {
    let args = Args::parse();
    match args.command {
        MainCommand::List => {
            let controllers = game_controllers().unwrap();
            if controllers.is_empty() {
                println!("No controllers found");
                return;
            }
            for item in controllers {
                println!("{:?}", item);
            }        
        },

        MainCommand::Enable { id } => {
            match enable_device(&id) {
                Ok(()) => {
                    println!("Device {} disabled successfully", &id)
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                }
            }
        },

        MainCommand::Disable { id } => {
            match disable_device(&id) {
                Ok(()) => {
                    println!("Device {} disabled successfully", &id)
                },
                Err(err) => {
                    println!("Error: {:?}", err);
                }
            }
        }
    }
}
