# Trace Tool

This is a simple command line utility that parses 
[chromium trace format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)
and provides some basic functions.

This was mostly done as an exercise to learn rust and scratch some niche needs
I had dealing with chrome traces.


# Commands

- **list**: List all processes included in the trace. For each process print a
  summary of their info including PID, name, type, and number of threads.



- **filter**: Produce a new trace that only includes the given renderer
  processes in addition to all other non-renderer processes. This is
  particularly useful when one collects a trace with a Chromium browser that
  have many open tabs (Renderer processes) but only wants to include the
  particular renderer for the given site.

# Sample output
Below is a screenshot of list output:
<img width="918" alt="trace_tool-list-screenshot" src="https://user-images.githubusercontent.com/944639/123524281-2db25600-d697-11eb-8192-0b1a0c94b659.png">

# Build & Usage

Once you have installed rust toolchain and cargo then simply:

``` 
$ cargo run -- --help
```

