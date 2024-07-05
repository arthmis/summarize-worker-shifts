### Installing and Running
Install Rust following this link https://www.rust-lang.org/tools/install.

cd to root of the project's directory
   ```cd summarize-worker-shifts```
Run the program with the given dataset
```
cargo run -- "dataset_(1).json"
```
Remember to put quotations around the file name or weird things might happen like a file read error. If you want to use a different file you can. Like so
```
cargo run -- "your_file_path_here"
```

#### Run optimized build
If you want to run the optimized build then
```
cargo run -- "your_file_path_here" --release
```

## Next Steps 
- I would add more tests
- Add tests that are more robust, meaning handle more edge cases especially concerning calculations around Central time and converting to/from UTC
- I would also consider deleting the unit tests I have for the helper functions that helped with summarizing and instead write more test cases that test `summarize_shifts_from_json_file`
- I would work out how to make the bonus feature work where I handle transition from CDT to CST and vice versa
- I would add errors that are more granular and potentially specific to the problem at hand to provide better context surrounding an error
  - such as json read error, if key names are not expected
  - or date is not in format expected
  - add more information on validation errors addressed below
- I would add more validation
  - check end time is after start time for the shift
  - create a type for employee_id and shift_id instead of passing around u64s
- Fix a performance issue when checking if shifts are valid
  - right now, for every shift I check all employee shifts and see if they overlap with the current shift
  - I can save time with memoization by checking a shift overlaps and if it does, take every shift that overlaps add them to the summaries and then store the overlapping shifts in a hashmap.
    Then I can search the hashmap when going to new shifts. If I get to a new shift that is in this hashmap of overlapping shifts, then I can skip it
- Might reconsider when I calculate overtime hours to when hours get added to summaries
