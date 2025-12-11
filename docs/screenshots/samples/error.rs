use std::collections::HashMap;

fn main() {
    let x: i32 = "hello";  // Type mismatch error

    let unused_var = 42;   // Unused variable warning

    println!("{}", undefined_var);  // Undefined variable

    let result = divide(10, 0);
}

fn divide(a: i32, b: i32) -> i32 {
    a / b  // Missing return for error case
}

struct User {
    name: String,
    age: u32
}

impl User {
    fn greet(&self) {
        println!("Hello, {}!", self.name);
    }
}
