
# Rust Hexchat API

This library provides a Rust API to the Hexchat Plugin Interface with additional
Rust friendly features such as:
* A thread-safe API.
* Simple `user_data` objects.
* Abstractions like `Context` that make it simple to interact with specific tabs/windows in the UI.
* Panic's are caught and displayed in the active Hexchat window.
* Debug builds include a full stack trace for panics.
* Hooked commands can be implemented as normal functions or closures.
* Typed preference values and easy plugin pref access.

## Documentation
Documentation can be found [here](https://ttappr.github.io/hexchat_api/hexchat_api/index.html).

## Example

Setting up and registering commands using the API is easy and syntactically 
clean. 

Interaction between threads and Hexchat is facilitated by `main_thread()`, which
uses Hexchat's timer event loop to delegate tasks, such as printing output
to the active Hexchat window.

```rust,no_run
hexchat.hook_command(
    "runthread",
    Priority::Norm,

    |hc, word, word_eol, ud| {
        // Spawn a new thread.
        thread::spawn(|| {
            // Send a task to the main thread to have executed and get its
            // AsyncResult object.
            let async_result = main_thread(|hc| {
                                        hc.print("Hello from main thread!");
                                        "This is the return value from main!"
                                   });
            // Get the return data from the main thread callback (blocks).
            let result = async_result.get();
            outpth!(hc, "Spawned thread received from main thread: {}", result);
        });
        Eat::All
    },

    "Runs a new thread that sets up a closure to run on the main thread.",
    NoData);
```

## Linking to `hexchat_api`

Simply include an entry in your Rust project's `Cargo.toml` file:

```toml
[dependencies]
hexchat_api = { git = "https://github.com/ttappr/hexchat_api.git", branch = "main" }
```

