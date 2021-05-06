mod alloc;
use alloc::{AlligatorAlloc,SizeClass,MIN_SIZE_CLASS,MAX_SIZE_CLASS,FreshReusedStats,AllocMetrics};
use alloc::heap::HeapType;
use core::alloc::Layout;
use std::alloc::GlobalAlloc;
use cfg_if::cfg_if;
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

fn print_usage() {
    println!("bench-alloc-report.rs - Perform benchmark allocations and output metrics

USAGE

    bench-alloc-report.rs [-h,--csv-header|--only-csv-header] <iterations> [<report interval>]

OPTIONS

    -h              Display help text
    --csv-header    Print CSV header row
    --only-csv-header    Print CSV header row and exit

ARGUMENTS

    <iterations>           The number of random iterations to perform.
    [<report interval>]    The number of iterations to wait before outputting a report line. Optional, defaults to 100.

BEHAVIOR

    Outputs the results as a CSV table row.
");
}

fn print_metrics(iteration: u64, metrics: AllocMetrics, ratio: FreshReusedStats, total_alloc_bytes: u32) {
    // Aggregate some fields
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
             iteration=iteration,
             total_alloc_bytes=total_alloc_bytes,
             total_minipages=metrics.total_minipages,
             heap_bytes_write=metrics.heap_bytes_write,
             heap_bytes_read=metrics.heap_bytes_read,
             total_allocs=total_allocs,
             total_deallocs=total_deallocs,
             fresh_allocs=fresh_allocs,
             reused_allocs=reused_allocs
    );
}

fn args_pop_u64(args: &mut Vec<String>, name: String) -> Result<u64, String> {
    match args.pop() {
        Some(a) => match a.parse::<u64>() {
            Ok(a) => Ok(a),
            Err(e) => Err(format!("error: {} argument \"{}\" could not be converted into u64: {}", name, a, e)),
        },
        None => Err(format!("error: {} argument required", name)),
    }
}

/// Allocate and free a lot of times.
#[cfg(feature = "metrics")]
fn main() {
    let mut rng = thread_rng();
    let mut free_later: Vec<*mut u8> = vec!(); // Free these addresses later

    // Used when a layout needs to be passed but it doesn't matter what its value is
    let dummy_layout = match Layout::from_size_align(8, 1) {
        Ok(l) => l,
        Err(e) => panic!("error making dummy Layout: {}", e),
    };
    
    // Parse command line arguments
    let mut args: Vec<String> = env::args().collect();
    args.reverse();
    let prog_name = args.pop().unwrap();
    args.reverse();

    if args.len() == 0 {
        eprintln!("error: incorrect arguments");
        print_usage();
        exit(1);
    }

    if args[0] == "-h" || args[0] == "--help" {
        print_usage();
        exit(0);
    } else if args[0] == "--csv-header" || args[0] == "--only-csv-header" {
        println!("iteration,total_alloc_bytes,total_minipages,heap_bytes_write,heap_bytes_read,total_allocs,total_deallocs,fresh_allocs,reused_allocs");

        if args[0] == "--only-csv-header" {
            return exit(0);
        }

        args.reverse();
        args.pop();
    }

    let arg_iterations = args_pop_u64(&mut args, "<iterations>".to_string()).unwrap();
    let arg_report_interval = match args.len() > 0 {
        true => args_pop_u64(&mut args, "<report interval>".to_string()).unwrap(),
        false => 100,
    };

    let mut total_alloc_bytes: u32 = 0;

    for iteration in 0..=arg_iterations {
        // Allocate
        let (metrics, ratio) = unsafe {
            // Choose random size to allocate.
            let alloc_bytes: u32 = rng.gen_range(2_u32.pow(u32::from(MIN_SIZE_CLASS))..=2_u32.pow(u32::from(MAX_SIZE_CLASS)));
            total_alloc_bytes += alloc_bytes;

            // Create layout which requests the maximum number of bytes possible for this size class
            let layout = match Layout::from_size_align(usize::try_from(alloc_bytes).unwrap(), 1) {
                Ok(l) => l,
                Err(e) => panic!("error making Layout::from_size_align({}, 1): {}", alloc_bytes, e),
            };

            // Call allocate
            let ptr = ALLOC.alloc(layout);

            // Either free immediately or free now
            let should_free_now: u8 = rng.gen_range(0..10);
            if should_free_now <= 4 {
                // Don't immediately free ~40% of allocations.
                free_later.push(ptr);
            } else {
                ALLOC.dealloc(ptr, layout);
            }

            // Determine if we should free one of the addresses which we left around
            let should_free_other_old: u8 = rng.gen_range(0..10);
            if free_later.len() > 0 && should_free_other_old <= 4 {
                // Free stuff from free_later about 40% of the time
                let free_idx: usize = rng.gen_range(0..free_later.len());
                let free_ptr = free_later[free_idx];
                ALLOC.dealloc(free_ptr, dummy_layout); // Using the wrong layout shouldn't matter
                free_later.remove(free_idx);
            }

            // Return metrics
            let metrics = match ALLOC.metrics() {
                Some(m) => m,
                None => panic!("no metrics found after allocations and deallocations were performed"),
            };

            let ratio = ALLOC.fresh_reused_stats();

            (metrics, ratio)
        };

        if iteration % arg_report_interval == 0 {
            print_metrics(iteration, metrics, ratio, total_alloc_bytes);
        }
    }

    let (metrics, ratio) = unsafe {
        // Free the memory we intentionally left laying around.
        for ptr in free_later.iter() {
            ALLOC.dealloc(*ptr, dummy_layout);
        }


        let metrics = match ALLOC.metrics() {
            Some(m) => m,
            None => panic!("no metrics found after allocations and deallocations were performed"),
        };

        let ratio = ALLOC.fresh_reused_stats();

        (metrics, ratio)
    };

    print_metrics(arg_iterations+1, metrics, ratio, total_alloc_bytes);
}
