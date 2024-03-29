# Hyprscratch

A scratchpad utility for Hyprland

## Installation
Using cargo (Make sure `~/.cargo/bin` is in $PATH):

```
cargo install hyprscratch
```

## Usage
In `hyprland.conf`:

```bash
exec-once = ~/.cargo/bin/hyprscratch [OPTIONS]

bind = $MOD, $KEY, exec, ~/.cargo/bin/hyprscratch $WINDOW_TITLE "$HYPRLAND_EXEC_COMMAND" [OPTIONS]
```

For example:

```bash
bind = $mainNod, b, exec, ~/.cargo/bin/hyprscratch btop "[float;size 70% 80%;center] kitty -e btop"
```

You can use the `stack` option so that the new scratchpad doesn't hide the old one. This can be used to group multiple scratchpads by binding them to the same key and using `stack` on all except the first one. 

If you like stacking scratchpads, there is a command `hyprscratch hideall` that you can call to hide all scratchpads. 

If you want a scratchpad to spawn on startup, you can add `onstart` as an option.

There is also a `hyprscratch cycle` command that cycles between scratchpads in the order in which they are defined in the config.

The scratchpads are just floating windows so by default they remain on the workspace they are spawned if not explicitly hidden. To hide them on workspace change add:

```bash
exec-once = ~/.cargo/bin/hyprscratch clean
```

To also hide when losing focus to a non-floating window:
```bash
exec-once = ~/.cargo/bin/hyprscratch clean spotless
```

You can use the `shiny` option to prevent a specific scratchpad from being cleaned on focus change. It is useful for graphical program where you would want to drag and drop.

In order for the program to funtion properly after making changes to the config, you can use `hyprscratch reload` to reparse the config without restarting. Adding the `shiny` option to an existing scratchpad will not work with `reload` and requires the program to restarted.

## Other Relevant information
If you have any issues with windows not spawning, try using `hyrscratch get-config` to see if your commands are being parsed propery.

To find the title needed for a scratchpad, run `hyprctl clients` and check the `initialTitle` field. An incorrect title results in the scratchpad not being hidden and a new one being spawned instead.

If there are multiple scratchpads with the same initial title, the program just grabs the first one it finds.

Scratchpads don't have to be floating. This can also be used to just spawn a specific window, where the binding also hides it or grabs it from another workspace. Non-floating scratchpads don't get cleaned.

If you want a scratchpad centered properly, the `center` option needs to be last.

The program doesn't use Hyprland's special workspace, it uses workspace 42.
