use hyprland::data::{Client, Clients, Workspace};
use hyprland::dispatch::*;
use hyprland::event_listener::EventListenerMutable;
use hyprland::prelude::*;
use hyprland::Result;
use regex::Regex;
use std::fs::remove_file;
use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

fn summon(title: &str, cmd: &str) -> Result<()> {
    let clients_with_title = &Clients::get()?
        .filter(|x| x.initial_title == title)
        .collect::<Vec<_>>();

    if clients_with_title.is_empty() {
        hyprland::dispatch!(Exec, cmd)?;
    } else {
        let pid = clients_with_title[0].pid as u32;
        if clients_with_title[0].workspace.id == Workspace::get_active()?.id {
            hyprland::dispatch!(FocusWindow, WindowIdentifier::ProcessId(pid))?;
        } else {
            hyprland::dispatch!(
                MoveToWorkspaceSilent,
                WorkspaceIdentifierWithSpecial::Relative(0),
                Some(WindowIdentifier::ProcessId(pid))
            )?;
            hyprland::dispatch!(FocusWindow, WindowIdentifier::ProcessId(pid))?;
        }
        Dispatch::call(DispatchType::BringActiveToTop)?;
    }
    Ok(())
}

fn scratchpad(title: &str, args: &[String]) -> Result<()> {
    let mut stream = UnixStream::connect("/tmp/hyprscratch.sock")?;
    stream.write_all(b"\0")?;

    let mut titles = String::new();
    stream.read_to_string(&mut titles)?;

    let cl = Client::get_active()?;
    match cl {
        Some(cl) => {
            if (!args.contains(&"stack".to_string()) && (cl.floating && titles.contains(&cl.title)))
                || cl.initial_title == title
            {
                hyprland::dispatch!(
                    MoveToWorkspaceSilent,
                    WorkspaceIdentifierWithSpecial::Id(42),
                    None
                )?;
            }

            if cl.initial_title != title {
                summon(title, &args[0])?;
            }
        }
        None => summon(title, &args[0])?,
    }
    Ok(())
}

fn reload() -> Result<()> {
    let mut stream = UnixStream::connect("/tmp/hyprscratch.sock")?;
    stream.write_all(b"r")?;
    Ok(())
}

fn hideall() -> Result<()> {
    Clients::get()?
        .filter(|x| x.floating && x.workspace.id == Workspace::get_active().unwrap().id)
        .for_each(|x| {
            hyprland::dispatch!(
                MoveToWorkspaceSilent,
                WorkspaceIdentifierWithSpecial::Id(42),
                Some(WindowIdentifier::ProcessId(x.pid as u32))
            )
            .unwrap()
        });
    Ok(())
}

fn dequote(string: &String) -> String {
    let dequoted = match &string[0..1] {
        "\"" | "'" => &string[1..string.len() - 1],
        _ => string,
    };
    dequoted.to_string()
}

fn parse_config() -> [Vec<String>; 3] {
    let lines_with_hyprscratch_regex = Regex::new("hyprscratch.+").unwrap();
    let hyprscratch_args_regex = Regex::new("\".+?\"|'.+?'|\\w+").unwrap();
    let mut buf: String = String::new();

    let mut titles: Vec<String> = Vec::new();
    let mut commands: Vec<String> = Vec::new();
    let mut options: Vec<String> = Vec::new();

    std::fs::File::open(format!(
        "{}/.config/hypr/hyprland.conf",
        std::env::var("HOME").unwrap()
    ))
    .unwrap()
    .read_to_string(&mut buf)
    .unwrap();

    let lines: Vec<&str> = lines_with_hyprscratch_regex
        .find_iter(&buf)
        .map(|x| x.as_str())
        .collect();

    for line in lines {
        let parsed_line = &hyprscratch_args_regex
            .find_iter(line)
            .map(|x| x.as_str().to_string())
            .collect::<Vec<_>>()[..];

        if parsed_line.len() == 1 {
            continue;
        }

        match parsed_line[1].as_str() {
            "clean" | "hideall" => (),
            _ => {
                titles.push(dequote(&parsed_line[1]));
                commands.push(dequote(&parsed_line[2]));

                if parsed_line.len() > 3 {
                    options.push(parsed_line[3..].join(" "));
                } else {
                    options.push(String::from(""));
                }
            }
        };
    }
    [titles, commands, options]
}

fn move_floating(titles: Vec<String>) {
    if let Ok(clients) = Clients::get() {
        clients
            .filter(|x| x.floating && x.workspace.id != 42 && titles.contains(&x.initial_title))
            .for_each(|x| {
                hyprland::dispatch!(
                    MoveToWorkspaceSilent,
                    WorkspaceIdentifierWithSpecial::Id(42),
                    Some(WindowIdentifier::ProcessId(x.pid as u32))
                )
                .unwrap()
            })
    }
}

fn clean(
    cli_options: &[String],
    titles: &[String],
    options: &[String],
) -> Result<()> {
    let mut ev = EventListenerMutable::new();

    let titles_clone = titles.to_owned();
    let unshiny_titles: Vec<String> = titles
        .iter()
        .cloned()
        .enumerate()
        .filter(|&(i, _)| !options[i].contains("shiny"))
        .map(|(_, x)| x)
        .collect();

    ev.add_workspace_change_handler(move |_, _| {
        move_floating(titles_clone.clone());
    });

    if cli_options.contains(&"spotless".to_string()) {
        ev.add_active_window_change_handler(move |_, _| {
            if let Some(cl) = Client::get_active().unwrap() {
                if !cl.floating {
                    move_floating(unshiny_titles.clone());
                }
            }
        });
    }
    std::thread::spawn(|| ev.start_listener());
    Ok(())
}

fn autospawn(titles: &[String], commands: &[String], options: &[String]) {
    let clients = Clients::get()
        .unwrap()
        .map(|x| x.initial_title)
        .collect::<Vec<_>>();

    commands
        .iter()
        .enumerate()
        .filter(|&(i, _)| options[i].contains("onstart") && !clients.contains(&titles[i]))
        .for_each(|(_, x)| {
            hyprland::dispatch!(Exec, &x.replacen('[', "[workspace 42 silent;", 1)).unwrap()
        });
}

fn get_config() {
    let [titles, commands, options] = parse_config();
    let max_len = |xs: &Vec<String>| xs.iter().map(|x| x.chars().count()).max().unwrap();
    let padding = |x: usize, y: usize| " ".repeat(x - y);

    let max_titles = max_len(&titles);
    let max_commands = max_len(&commands);
    let max_options = max_len(&options);

    for i in 0..titles.len() {
        println!(
            "\x1b[0;31mTitle:\x1b[0;31m {}{}  \x1b[0;31mCommand:\x1b[0;1m {}{}  \x1b[0;31mOptions:\x1b[0;0m {}{}",
            titles[i],
            padding(max_titles, titles[i].chars().count()),
            commands[i],
            padding(max_commands, commands[i].chars().count()),
            options[i],
            padding(max_options, options[i].chars().count())
        )
    }
}

fn handle_stream(
    stream: &mut UnixStream,
    current_titles: &mut Vec<String>,
    args: &[String],
) -> Result<()> {
    let mut buf = String::new();
    stream.try_clone()?.take(1).read_to_string(&mut buf)?;

    match buf.as_str() {
        "r" => {
            let [titles, commands, options] = parse_config();
            *current_titles = titles.clone();
            autospawn(&titles, &commands, &options);
            clean(args, &titles, &options)?;
        }
        "\0" => {
            let titles_string = format!("{current_titles:?}");
            stream.write_all(titles_string.as_bytes())?;
        }
        b => println!("Unknown request {b}"),
    }
    Ok(())
}

fn initialize(title: &str, args: &[String]) -> Result<()> {
    let mut cli_args = args.join(" ");
    cli_args.push_str(title);

    let [mut titles, commands, options] = parse_config();
    autospawn(&titles, &commands, &options);

    if cli_args.contains("clean") {
        clean(args, &titles, &options, )?;
    }

    let path_to_sock = Path::new("/tmp/hyprscratch.sock");
    if path_to_sock.exists() {
        remove_file(path_to_sock)?;
    }

    let listener = UnixListener::bind(path_to_sock)?;
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                handle_stream(
                    &mut stream,
                    &mut titles,
                    args,
                )?;
            }
            Err(_) => {
                break;
            }
        };
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<String>>();
    let title = match args.len() {
        0 | 1 => String::from(""),
        2.. => args[1].clone(),
    };

    let empty_sting_array = [String::new()];
    let cli_args = match args.len() {
        0..=2 => &empty_sting_array,
        3.. => &args[2..],
    };

    match title.as_str() {
        "clean" | "" => initialize(&title, cli_args)?,
        "get-config" => get_config(),
        "hideall" => hideall()?,
        "reload" => reload()?,
        _ => scratchpad(&title, cli_args)?,
    }
    Ok(())
}
