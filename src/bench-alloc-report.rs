mod alloc;
use alloc::{AlligatorAlloc,SizeClass,MIN_SIZE_CLASS,MAX_SIZE_CLASS};
use alloc::heap::HeapType;
use core::alloc::Layout;
use std::alloc::GlobalAlloc;
use cfg_if::cfg_if;
use rand::prelude::*;
use std::env;
use std::process::exit;

/*
 * What follows is the benchmark program. Right now it
 * just tries to get any sort of memory allocation
 * to occur. Comment out annotation but keep variable if 
 * debugging ALLOC crashes further.
 */
// #[global_allocator]
static ALLOC: AlligatorAlloc<HeapType> = AlligatorAlloc::INIT;

/// Allocate then free a specified number of segments for a size class. A certain number of segments will be randomly not freed until the end of the function, while the rest of the segments will be freed immediately after allocation.
///
/// # Panics
/// If the allocation or deallocation process fails.
#[cfg(feature = "metrics")]
unsafe fn alloc_segments(size_class_exp: u8, num_segments_factor: u32) {
    let size_class = SizeClass::new(size_class_exp);
    
    let mut free_later: Vec<*mut u8> = vec!();

    for i in 0..u32::from(size_class.segments_max_num()) * num_segments_factor {
        
        // Create layout which requests the maximum number of bytes possible for this size class
        let layout = match Layout::from_size_align(2_usize.pow(u32::from(size_class_exp)), 1) {
            Ok(l) => l,
            Err(e) => panic!("error making Layout: {}", e),
        };

        // Call allocate
        let ptr = ALLOC.alloc(layout);
        
        // Ensure the allocation succeeded
        assert!(!ptr.is_null(), "alloc() failed (returned null): size class={}, i={}, alloc failure cause={:?}", size_class_exp, i, ALLOC.alloc_failure_cause());

        // For 1/6th of allocations don't free them immediately, free them later
        let mut rng = thread_rng();
        let rand: u32 = rng.gen_range(0..6);
        
        if rand == 0 {
            free_later.push(ptr);
        } else {
            ALLOC.dealloc(ptr, layout);
        }
    }

    // Free the memory we intentionally left laying around.
    for ptr in free_later.iter() {
        let layout = match Layout::from_size_align(2_usize.pow(u32::from(size_class_exp)), 1) {
            Ok(l) => l,
            Err(e) => panic!("error making Layout: {}", e),
        };
        
        ALLOC.dealloc(*ptr, layout);
    }
}

fn print_usage() {
    println!("bench-alloc-report.rs - Perform benchmark allocations and output metrics

USAGE

    bench-alloc-report.rs [-h,--csv-header] <min size class> <max size class> <min pages> <max pages>

OPTIONS

    -h              Display help text
    --csv-header    Print CSV header row
    --only-csv-header    Print CSV header row and exit

ARGUMENTS

    Size class: <min> <max>    Inclusive range of size class exponents to allocate (n^<size class> bytes)
    Pages: <min> <max>         Inclusive range of the number of MiniPages to allocate

BEHAVIOR

    Inputs are of inclusive ranges of unsigned 8 bit integers in the format:

        <min> <max>

    Outputs the results as a CSV table row.
");
}

fn args_pop_u8(args: &mut Vec<String>, name: String) -> Result<u8, String> {
    match args.pop() {
        Some(a) => match a.parse::<u8>() {
            Ok(a) => Ok(a),
            Err(e) => Err(format!("error: {} argument \"{}\" could not be converted into u8: {}", name, a, e)),
        },
        None => Err(format!("error: {} argument required", name)),
    }
}

/// Allocate and free a lot of times.
#[cfg(feature = "metrics")]
fn main() {
    // Parse command line arguments
    let mut args: Vec<String> = env::args().collect();
    args.reverse();
    let prog_name = args.pop().unwrap();

    if args.len() == 0 {
        eprintln!("error: incorrect arguments");
        print_usage();
        exit(1);
    }

    if args[0] == "-h" {
        print_usage();
        exit(0);
    } else if args[0] == "--csv-header" || args[0] == "--only-csv-header" {
        println!("size_class,total_minipages,heap_bytes_write,heap_bytes_read,total_allocs,total_deallocs,fresh_allocs,reused_allocs");

        if args[0] == "--only-csv-header" {
            return exit(0);
        }

        args.pop();
    }
    
    let arg_size_class_min = args_pop_u8(&mut args, "<size class> minimum".to_string()).unwrap();
    let arg_size_class_max = args_pop_u8(&mut args, "<size class> maximum".to_string()).unwrap();
    let arg_segments_factor_min = args_pop_u8(&mut args, "<pages> minimum".to_string()).unwrap();
    let arg_segments_factor_max = args_pop_u8(&mut args, "<pages> maximum".to_string()).unwrap();

    for arg_size_class in arg_size_class_min..=arg_size_class_max {
        for arg_segments_factor in arg_segments_factor_min..=arg_segments_factor_max {
            // Allocate
            let (metrics, ratio) = unsafe {
                alloc_segments(arg_size_class, u32::from(arg_segments_factor));

                let metrics = match ALLOC.metrics() {
                    Some(m) => m,
                    None => {
                        panic!("no metrics found after allocations and deallocations were performed");
                    },
                };

                let ratio = ALLOC.fresh_reused_stats();

                (metrics, ratio)
            };

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
            println!("{size_class},{total_minipages},{heap_bytes_write},{heap_bytes_read},{total_allocs},{total_deallocs},{fresh_allocs},{reused_allocs}",
                     size_class=arg_size_class,
                     total_minipages=metrics.total_minipages,
                     heap_bytes_write=metrics.heap_bytes_write,
                     heap_bytes_read=metrics.heap_bytes_read,
                     total_allocs=total_allocs,
                     total_deallocs=total_deallocs,
                     fresh_allocs=fresh_allocs,
                     reused_allocs=reused_allocs
            );
        }
    }
}
