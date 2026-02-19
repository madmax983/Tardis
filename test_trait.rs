trait Foo {
    fn name(&self) -> &str;
}

struct Bar;

impl Foo for Bar {
    fn name(&self) -> &'static str {
        "bar"
    }
}

fn main() {}
