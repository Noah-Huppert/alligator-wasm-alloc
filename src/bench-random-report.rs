mod alloc;
use alloc::{AlligatorAlloc,SizeClass,MIN_SIZE_CLASS,MAX_SIZE_CLASS};
use alloc::heap::HeapType;
use core::alloc::Layout;
use std::alloc::GlobalAlloc;
use rand::prelude::*;
use std::env;
use std::process::exit;
use std::convert::TryFrom;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur. Comment out annotation but keep variable if 
 * debugging ALLOC crashes further.
 */
// #[global_allocator]
static ALLOC: AlligatorAlloc<HeapType> = AlligatorAlloc::INIT;

/// Implements a pattern of randomly allocation and freeing.
struct RandomReport {
    /// Thread random number generator.
    rng: ThreadRng,

    /// Pointers which should be freed later.
    free_later: Vec<*mut u8>,
    
    /// The number of times the benchmark has performed the main loop.
    iteration: u64,

    /// The total number of bytes which have been allocated.
    total_alloc_bytes: u64,

    /// Range of size classes which are allowed to be allocated.
    alloc_range: InclusiveRange<u8>,
}

impl RandomReport {
    /// Prints a CSV data row based on the current allocator metrics.
    unsafe fn print_metrics(&mut self) {
        // Return metrics
        let metrics = match ALLOC.metrics() {
            Some(m) => m,
            None => panic!("no metrics found after allocations and deallocations were performed"),
        };

        let ratio = ALLOC.fresh_reused_stats();
        
        // Aggregate per size class metrics into totals
        let mut total_allocs = 0;
        let mut total_deallocs = 0;
        
        let mut fresh_allocs = 0;
        let mut reused_allocs = 0;
        
        for i in MIN_SIZE_CLASS..=MAX_SIZE_CLASS {
            let size_class = SizeClass::new(i);
            
            total_allocs += metrics.total_allocs[size_class.exp_as_idx()];
            total_deallocs += metrics.total_deallocs[size_class.exp_as_idx()];

            fresh_allocs += ratio.total_alloc_fresh[size_class.exp_as_idx()];
            reused_allocs += ratio.total_alloc_reused[size_class.exp_as_idx()];
        }

        // Print results in a CSV table
        println!("{iteration},{total_alloc_bytes},{total_minipages},{heap_bytes_write},{heap_bytes_read},{total_allocs},{total_deallocs},{fresh_allocs},{reused_allocs}",
                 iteration=self.iteration,
                 total_alloc_bytes=self.total_alloc_bytes,
                 total_minipages=metrics.total_minipages,
                 heap_bytes_write=metrics.heap_bytes_write,
                 heap_bytes_read=metrics.heap_bytes_read,
                 total_allocs=total_allocs,
                 total_deallocs=total_deallocs,
                 fresh_allocs=fresh_allocs,
                 reused_allocs=reused_allocs
        );
    }

    /// Allocate a random size.
    unsafe fn iterate(&mut self) {
        // Choose random size to allocate.
        let alloc_bytes: u64 = self.rng.gen_range(2_u64.pow(u32::from(self.alloc_range.min))..=2_u64.pow(u32::from(self.alloc_range.max)));
        self.total_alloc_bytes += alloc_bytes;

        // Create layout which requests the maximum number of bytes possible for this size class
        let layout = match Layout::from_size_align(usize::try_from(alloc_bytes).unwrap(), 1) {
            Ok(l) => l,
            Err(e) => panic!("error making Layout::from_size_align({}, 1): {}", alloc_bytes, e),
        };

        // Call allocate
        let ptr = ALLOC.alloc(layout);

        if ptr.is_null() {
            panic!("alloc({}) failed: {:?}", alloc_bytes, ALLOC.alloc_failure_cause());
        }

        // Either free immediately or free at a random later iteration
        let should_free_now: u8 = self.rng.gen_range(0..10);
        if should_free_now <= 4 {
            // Don't immediately free ~40% of allocations.
            self.free_later.push(ptr);
        } else {
            ALLOC.dealloc(ptr, layout);
        }

        // Determine if we should free one of the addresses which we left around to free at another random time
        let should_free_other_old: u8 = self.rng.gen_range(0..10);
        if self.free_later.len() > 0 && should_free_other_old <= 1 {
            // Free stuff from free_later about 40% of the time
            let free_idx: usize = self.rng.gen_range(0..self.free_later.len());
            let free_ptr = self.free_later[free_idx];
            ALLOC.dealloc(free_ptr, layout); // Using the wrong layout shouldn't matter
            self.free_later.remove(free_idx);
        }

        self.iteration += 1;
    }

    /// Cleanup any remaining allocations which were left. Then print a final line of metrics so we can confirm everything is clean.
    unsafe fn cleanup(&mut self) {
        // Used when a layout needs to be passed but it doesn't matter what its value is
        let dummy_layout = match Layout::from_size_align(8, 1) {
            Ok(l) => l,
            Err(e) => panic!("error making dummy Layout: {}", e),
        };
        
        // Free the memory we intentionally left laying around.
        for ptr in self.free_later.iter() {
            ALLOC.dealloc(*ptr, dummy_layout);
        }

        self.print_metrics();
    }
}

/// Behavior of printing the CSV header
enum PrintCSVHeader {
    /// Print and continue running the benchmark.
    Continue,

    /// Print then exit.
    Exit,
}

/// An inclusive range.
struct InclusiveRange<T> {
    /// Minimum.
    min: T,

    /// Maximum.
    max: T,
}

/// Program run arguments.
struct Args {
    /// If true will print help text and exit.
    print_usage: Option<bool>,
    
    /// The number of iterations to perform.
    max_iterations: Option<u64>,

    /// The interval of iterations to print reports.
    report_interval: Option<u64>,

    /// If the CSV header should be printed.
    print_csv_header: Option<PrintCSVHeader>,

    /// Defines the range of size classes to allocate.
    alloc_range: Option<InclusiveRange<u8>>,

    /// If program should print a dot graphviz representation of the allocator internal state.
    print_dot_graph: Option<()>,
}

impl Args {
    /// Parse arguments from command line input. Destroys args argument.
    fn new(args: &mut Vec<String>) -> Args {
        let mut parsed = Args{
            print_usage: None,
            max_iterations: None,
            report_interval: None,
            print_csv_header: None,
            alloc_range: None,
            print_dot_graph: None,
        };
        
        while !args.is_empty() {
            let arg = args.pop().unwrap();

            if arg == "-h" || arg == "--help" {
                parsed.print_usage = Some(true);
            } else if arg == "-i" || arg == "--max-iterations" {
                parsed.max_iterations = Some(args.pop().unwrap().parse().unwrap());
            } else if arg == "-r" || arg == "--report-interval" {
                parsed.report_interval = Some(args.pop().unwrap().parse().unwrap());
            } else if arg == "-c" || arg == "--csv-header" {
                parsed.print_csv_header = Some(PrintCSVHeader::Continue);
            } else if arg == "-C" || arg == "--only-csv-header" {
                parsed.print_csv_header = Some(PrintCSVHeader::Exit);
            } else if arg == "-a" || arg == "--alloc" {
                parsed.alloc_range = Some(InclusiveRange::<u8>{
                    min: args.pop().unwrap().parse().unwrap(),
                    max: args.pop().unwrap().parse().unwrap(),
                });
            } else if arg == "-d" || arg == "dot-graph" {
                parsed.print_dot_graph = Some(());
            } else {
                panic!("unknown argument: {}", arg);
            }
        }

        // Set defaults
        if parsed.max_iterations.is_none() {
            parsed.max_iterations = Some(1000);
        }

        if parsed.report_interval.is_none() {
            parsed.report_interval = Some(100);
        }

        if parsed.alloc_range.is_none() {
            parsed.alloc_range = Some(InclusiveRange::<u8>{
                min: MIN_SIZE_CLASS,
                max: MAX_SIZE_CLASS,
            });
        }

        return parsed;
    }

    /// Print usage help text.
    fn print_usage() {
        println!("bench-random-report.rs - Perform random allocations and print metrics as CSV rows

USAGE

    bench-alloc-report.rs [-h] [-i,--max-iterations <num>] [-r,--report-interval <num>] [-d,--dot-graph] [-c,--csv-header] [-C,--only-csv-header] [-a,--alloc <min> <max>]

OPTIONS

    -h                            Display help text
    -i,--max-iteration <num>      Number of iterations to run (default 1000)
    -r,--report-interval <num>    The interval on which to print CSV metric rows (default 100)
    -d,--dot-graph                Print a dot graph of the allocator state.
    -a,--alloc <min> <max>        The, inclusive, minimum and maximum size class which can be randomly allocated (default {min_size_class} {max_size_class})
    -c,--csv-header               Print CSV header row first
    -C,--only-csv-header          Print CSV header row and exit

BEHAVIOR

    Randomly allocates bytes and outputs metrics as CSV table rows.

", min_size_class=MIN_SIZE_CLASS, max_size_class=MAX_SIZE_CLASS);
    }
}

/// Allocate and free a lot of times.
#[cfg(feature = "metrics")]
fn main() {
    // Parse command line arguments
    let mut args: Vec<String> = env::args().collect();
    args.reverse();
    args.pop().unwrap(); // Remove binary name
    
    let parsed_args = Args::new(&mut args);

    if let Some(print) = parsed_args.print_usage {
        if print {
            Args::print_usage();
            exit(0);
        }
    }

    if let Some(status) = parsed_args.print_csv_header {
        println!("iteration,total_alloc_bytes,total_minipages,heap_bytes_write,heap_bytes_read,total_allocs,total_deallocs,fresh_allocs,reused_allocs");
        
        match status {
            PrintCSVHeader::Exit => exit(0),
            _ => {},
        }
    }

    // Run benchmark
    let mut benchmark = RandomReport{
        rng: thread_rng(),
        free_later: vec!(),
        iteration: 0,
        total_alloc_bytes: 0,
        alloc_range: parsed_args.alloc_range.unwrap(),
    };

    for _i in 0..=parsed_args.max_iterations.unwrap() {
        unsafe {
            benchmark.iterate();
        }

        // Print metrics
        if benchmark.iteration % parsed_args.report_interval.unwrap() == 0 {
            unsafe {
                benchmark.print_metrics();
            }
        }
    }

    unsafe {
        benchmark.cleanup();
    }

    if let Some(_v) = parsed_args.print_dot_graph {
        unsafe {
            println!("dot graph:\n{}", ALLOC.dot_graph());
        }
    }
}
