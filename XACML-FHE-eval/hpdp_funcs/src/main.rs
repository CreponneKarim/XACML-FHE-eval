use hpdp_funcs::{UserConfig, bencher};
use clap::{Parser, Subcommand, Args};

#[derive(Parser, Debug)]
#[command(name = "XACML-FHE-EVAL")]
#[command(about = "Small CLI with bench command and params")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a benchmark
    Bench {
		/// Number of minimum iterations per one operation during the evaluation test. default is 5
		#[arg(short, long, default_value_t = 5)]
		min_iterations: usize,
		/// Number of maximum iterations per one operation during the evaluation test. default is 100
		#[arg(short, long, default_value_t = 100)]
		max_iterations: usize,
		/// percentage of error fluctuation threshold to stop default is 2%
		#[arg(short, long, default_value_t = 2)]
		rse: usize,
	},
}

fn main() {
    let cli = Cli::parse();
    println!("inside pdp");
    match cli.command {
        Commands::Bench { min_iterations, max_iterations, rse } => {
				bencher::bench_ops::bench_ops(min_iterations, max_iterations, (rse as f64)/100.0);
        }
	}
}
