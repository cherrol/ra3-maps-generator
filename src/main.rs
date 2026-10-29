mod cli;

fn main() {
    if let Err(e) = cli::run() {
        eprintln!("错误：{e}");
        std::process::exit(1);
    }
}
