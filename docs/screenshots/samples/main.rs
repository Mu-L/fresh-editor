use std::collections::HashMap;

/// A simple user management system
struct User {
    name: String,
    email: String,
    is_active: bool,
}

impl User {
    fn new(name: &str, email: &str) -> Self {
        User {
            name: name.to_string(),
            email: email.to_string(),
            is_active: true,
        }
    }

    fn greet(&self) -> String {
        format!("Hello, {}!", self.name)
    }
}

fn main() {
    let mut users: HashMap<u32, User> = HashMap::new();

    users.insert(1, User::new("Alice", "alice@example.com"));
    users.insert(2, User::new("Bob", "bob@example.com"));

    for (id, user) in &users {
        println!("[{}] {}", id, user.greet());
    }
}
