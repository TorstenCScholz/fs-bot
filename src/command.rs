use discord::{Connection};
use discord::model::{ServerId, UserId};

use std::cmp::{PartialEq, Eq};
use std::hash::{Hash, Hasher};

pub struct Command<'a> {
    name: &'a str,
    callback: Box<Fn(&mut Connection, &ServerId, &[&str]) -> ()>,
    requires_permission: bool
}

impl <'a> Command<'a> {
    pub fn new_default<'r>(name: &'r str, callback: Box<Fn(&mut Connection, &ServerId, &[&str]) -> ()>) -> Command<'r> {
        Command::new(name, callback, false)
    }

    pub fn new<'r>(name: &'r str, callback: Box<Fn(&mut Connection, &ServerId, &[&str]) -> ()>, req_perm: bool) -> Command<'r> {
        Command {
            name: name,
            callback: callback,
            requires_permission: req_perm
        }
    }

    pub fn get_name(&self) -> &str {
        self.name
    }

    pub fn is_permission_required(&self) -> bool {
        self.requires_permission
    }

    pub fn invoke(&self, connection: &mut Connection, server_id: &ServerId, user_id: &UserId, parameters: &[&str]) {
        println!("[Info] Invoking command {} with parameters {:?}.", self.get_name(), parameters);
        if !self.is_permission_required() {
            (self.callback)(connection, server_id, parameters);
        }
    }

    pub fn matches(&self, name: &str) -> bool {
        self.name == name
    }
}

impl <'a> PartialEq for Command<'a> {
    fn eq(&self, other: &Command) -> bool {
        self.name == other.name
    }
}

impl <'a> Eq for Command<'a> {}

impl <'a> Hash for Command<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}
