// use clap::Parser;
use std::{ ffi::OsStr, process::{ Child, Command, ExitStatus, Stdio }, time::Duration };

const POLKADOT_OMNI_NODE_BIN: &str = "polkadot-omni-node";
const CHAIN_SPEC_BUILDER: &str = "chain-spec-builder";
const ETH_RPC_BIN: &str = "eth-rpc";

#[derive(Clone)]
enum GitInstallType {
    Tag,
    CommitHash,
}

#[derive(Clone)]
struct GitOptions {
    url: String,
    tag_or_hash: String,
    install_type: GitInstallType,
}

struct Dependency {
    bin: String,
    install_bin: String,
    git: Option<GitOptions>,
}

impl Dependency {
    fn new(bin: &str, install_bin: &str, git: Option<GitOptions>) -> Self {
        Dependency { bin: bin.to_string(), install_bin: install_bin.to_string(), git }
    }
}

impl GitOptions {
    fn new(url: &str, tag: &str, install_type: GitInstallType) -> Self {
        GitOptions { url: url.to_string(), tag_or_hash: tag.to_string(), install_type }
    }
}

fn generate_child_process<I, S>(bin_name: S, args: I) -> Result<Child, std::io::Error>
    where I: IntoIterator<Item = S>, S: AsRef<OsStr>
{
    Command::new(bin_name).args(args).spawn()
}

fn kill_process(id: u32) -> Result<ExitStatus, std::io::Error> {
    generate_child_process("kill", ["-s", "TERM", &id.to_string()])?.wait()
}

fn install_dependency(dep: Dependency) -> Result<(), std::io::Error> {
    let git: Option<GitOptions> = dep.git;
    if let Some(git) = git {
        match git.install_type {
            GitInstallType::Tag => {
                let _ = generate_child_process("cargo", [
                    "install",
                    "--git",
                    &git.url,
                    "--tag",
                    &git.tag_or_hash,
                    &dep.install_bin,
                ])?.wait();
                return Ok(());
            }
            GitInstallType::CommitHash => {
                let _ = generate_child_process("cargo", [
                    "install",
                    "--git",
                    &git.url,
                    "--rev",
                    &git.tag_or_hash,
                    &dep.install_bin,
                ])?.wait();
                return Ok(());
            }
        }
    } else {
        let _ = generate_child_process("cargo", ["install", &dep.install_bin])?.wait();
    }
    Ok(())
}

fn check_dependencies(dependencies: Vec<Dependency>) -> Result<(), std::io::Error> {
    dependencies.into_iter().for_each(|dep| {
        if Command::new(&dep.bin).stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_err() {
            install_dependency(dep).expect("Could not install dependency");
        } else {
            println!("{} IS INSTALLED!", dep.bin);
        }
    });
    Ok(())
}

fn main() -> Result<(), std::io::Error> {
    let git_options = GitOptions::new(
        "https://github.com/paritytech/polkadot-sdk.git",
        "polkadot-stable2412",
        GitInstallType::Tag
    );

    // Make sure everything is installed
    println!("Checking dependencies");
    let dependencies = vec![
        Dependency::new(POLKADOT_OMNI_NODE_BIN, POLKADOT_OMNI_NODE_BIN, Some(git_options.clone())),
        Dependency::new(
            CHAIN_SPEC_BUILDER,
            "staging-chain-spec-builder",
            Some(git_options.clone())
        ),
        Dependency::new(
            ETH_RPC_BIN,
            "pallet-revive-eth-rpc",
            Some(
                GitOptions::new(
                    "https://github.com/paritytech/polkadot-sdk.git",
                    "d1d92ab76004ce349a97fc5d325eaf9a4a7101b7",
                    GitInstallType::CommitHash
                )
            )
        )
    ];

    check_dependencies(dependencies)?;
    //Generate chain-spec from params
    println!("Generating chain spec...");
    let _chain_spec = generate_child_process(CHAIN_SPEC_BUILDER, [
        "create",
        "--runtime",
        "./runtimes/westend.wasm",
        "--para-id",
        "100",
        "--relay-chain",
        "paseo",
        "named-preset",
        "development",
    ])?.wait()?;

    // Purge chain data
    println!("Purging previous chain data...");
    let _purge = generate_child_process(POLKADOT_OMNI_NODE_BIN, [
        "purge-chain",
        "--chain",
        "./chain_spec.json",
        "-y",
    ])?.wait()?;

    // Start the omninode
    let omni_node = generate_child_process(POLKADOT_OMNI_NODE_BIN, [
        "--chain",
        "./chain_spec.json",
        "--dev-block-time",
        "6000",
    ])?;

    // Start the ETH RPC
    let eth_rpc = generate_child_process(ETH_RPC_BIN, [
        "--chain",
        "./chain_spec.json",
        "--rpc-cors=all",
        "--log=debug",
    ])?;

    println!("🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋");
    println!("🤖🤖🤖 OMNINODE IS STARTING 🤖🤖🤖");
    println!("🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋 🐋");

    ctrlc
        ::set_handler(move || {
            kill_process(omni_node.id()).expect("Omninode failed to be killed");
            kill_process(eth_rpc.id()).expect("ETH RPC failed to be killed");
            println!("Carp finished 🐋");
            std::process::exit(0);
        })
        .expect("Error setting Ctrl-C handler");

    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
