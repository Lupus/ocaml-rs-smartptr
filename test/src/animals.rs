pub trait Animal {
    // Associated function signature; `Self` refers to the implementor type.
    fn new(name: String) -> Self;

    // Method signatures; these will return a string.
    fn name(&self) -> String;
    fn noise(&self) -> String;

    // Traits can provide default method definitions.
    fn talk(&self) {
        println!("{} says {}", self.name(), self.noise());
    }
}

pub struct Sheep {
    naked: bool,
    name: String,
}

impl Sheep {
    pub fn is_naked(&self) -> bool {
        self.naked
    }

    pub fn shear(&mut self) {
        if self.is_naked() {
            // Implementor methods can use the implementor's trait methods.
            println!("{} is already naked...", self.name());
        } else {
            println!("{} gets a haircut!", self.name);

            self.naked = true;
        }
    }
}

// Implement the `Animal` trait for `Sheep`.
impl Animal for Sheep {
    // `Self` is the implementor type: `Sheep`.
    fn new(name: String) -> Sheep {
        Sheep { name, naked: false }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn noise(&self) -> String {
        if self.is_naked() {
            "baaaaah?".into()
        } else {
            "baaaaah!".into()
        }
    }

    // Default trait methods can be overridden.
    fn talk(&self) {
        // For example, we can add some quiet contemplation.
        println!("{} pauses briefly... {}", self.name, self.noise());
    }
}

pub struct Wolf {
    name: String,
}

// Implement the `Animal` trait for `Wolf`.
impl Animal for Wolf {
    // `Self` is the implementor type: `Wolf`.
    fn new(name: String) -> Wolf {
        Wolf { name }
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn noise(&self) -> String {
        "rrrrrr!".into()
    }
}
