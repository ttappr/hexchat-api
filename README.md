
# Rust Hexchat API

This library provides a Rust API to the Hexchat Plugin Interface with additional
Rust friendly features such as:
* A thread-safe API.
* Simple `user_data` objects.
* Abstractions like `Context` that make it simple to interact with specific
  tabs/windows in the UI.
* Panic's are caught and displayed in the active Hexchat window.
* Debug builds include a full stack trace for panics.
* Hooked commands can be implemented as normal functions or closures.
* Typed preference values and easy plugin pref access.

## Documentation
Documentation can be found
[here](https://ttappr.github.io/hexchat-api/doc/hexchat_api/index.html)

## Examples

A completed plugin offers the most examples to pull from.
[Here's a plugin](https://github.com/ttappr/hexchat_translator) that does
automatic translations which enables chatting with people in different tongues
(chat with subtitles).

Setting up and registering commands using the API is easy and syntactically
clean.

Interaction between threads and Hexchat is facilitated by `main_thread()`, which
uses Hexchat's timer event loop to delegate tasks, such as printing output
to the active Hexchat window.

``` rust no_run
hexchat.hook_command(
    "runthread",
    Priority::Norm,

    |hc, word, word_eol, ud| {
        // Spawn a new thread.
        thread::spawn(|| {
            // Send a task to the main thread to have executed and
            // get its AsyncResult object.
            let async_result
                    = main_thread(|hc| {
                        hc.print("Hello from main thread!");

                        "This is the return value from main!"
                    });
            // Get the return data from the main thread callback
            // (blocks).
            let result = async_result.get();

            hc_print_th!("Spawned thread received from main \
                          thread: {}", result);
        });
        Eat::All
    },

    "Runs a new thread that sets up a closure to run on the main \
     thread.",
    NoData);
```

## Linking to `hexchat_api`

Simply include an entry in your Rust project's `Cargo.toml` file:

```toml
[dependencies]
hexchat-api = "0.3"
```

## Template

The code below can be copied to start a new plugin project. The TOML file
content is also included below.


``` rust no_run
// FILE: lib.rs

//! A starter project template that can be copied and modified.

use hexchat_api::*;
use UserData::*;

// Register the entry points of the plugin.
//
dll_entry_points!(plugin_info, plugin_init, plugin_deinit);

/// Called when the plugin is loaded to register it with Hexchat.
///
fn plugin_info() -> PluginInfo {
    PluginInfo::new(
        "Plugin Template",
        "0.1",
        "A Hexchat plugin to customize.")
}

/// Called when the plugin is loaded.
///
fn plugin_init(hc: &Hexchat) -> i32 {
    hc.print("Plugin template loaded");

    // Example user data to pass to a callback.
    let udata = UserData::boxed("Some data to pass to a callback.");

    // Register a simple command using a function.
    hc.hook_command("HELLOWORLD",
                    Priority::Norm,
                    hello_world,
                    "Prints \"Hello, world!\"",
                    NoData);

    // Register a simple command using a closure.
    hc.hook_command("HELLOHEX",
                    Priority::Norm,
                    |hc, word, word_eol, user_data| {

                        hc.print("Hello, Hexchat!");

                        user_data.apply(|msg: &&str| {
                            hc.print(msg);
                        });

                        Eat::All
                    },
                    "Prints \"Hello, Hexchat!\", and the user data.",
                    udata);
    1
}

/// Called when the plugin is unloaded.
///
fn plugin_deinit(hc: &Hexchat) -> i32 {
    hc.print("Plugin template unloaded");
    1
}

/// A command callback implemented as a function.
/// # Arguments
/// * `hc`        - The Hexchat API object reference.
/// * `word`      - A list of parameters passed to the command.
/// * `word_eol`  - Like `word`, but catenates the word args
///                 decrementally.
/// * `user_data` - The user data to be passed back to the command
///                 when invoked by Hexchat.
/// # Returns
/// * One of `Eat::All`, `Eat::Hexchat`, `Eat::Plugin`, `Eat::None`.
///
fn hello_world(hc        : &Hexchat,
               word      : &[String],
               word_eol  : &[String],
               user_data : &UserData
              ) -> Eat
{
    hc.print("Hello, world!");
    Eat::All
}
```

And the Cargo.toml file.

```toml
[package]
name = "hexchat_plugin_template"
version = "0.1.0"
authors = ["you <your@email.com>"]
edition = "2021"

[lib]
name = "hexchat_plugin_template"
crate-type = ["cdylib"]

[dependencies]
hexchat-api = "0.2"
```
