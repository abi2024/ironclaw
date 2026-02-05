wit_bindgen::generate!({
    world: "tool",
});

struct MyTool;

impl Guest for MyTool {
    fn run(input: String) -> String {
        // The Polite Logic: Just echo the input
        format!("(Guest) IronClaw processed: '{}'", input)
    }
}

export!(MyTool);