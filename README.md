# recursive_dir_walk

what perf we can squeeze from recursive directory walk

## usage

executed on directory with over 4 million entries overall
```bash
TEST_DIR=/path/to/dir
$ cargo build --release && echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null; time target/release/recursive_dir_walk $TEST_DIR | wc -l
    Finished release [optimized + debuginfo] target(s) in 0.10s
4080405

real    0m0.651s
user    0m0.488s
sys     0m3.350s

$ echo 3 | sudo tee /proc/sys/vm/drop_caches >/dev/null; time find $TEST_DIR | wc -l
4080405

real    0m6.219s
user    0m1.059s
sys     0m1.830s
```