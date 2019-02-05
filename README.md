# Trace Tool

This is a simple command line utility that parses 
[chromium trace format](https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview)
and provides some basic functions.

This was mostly done as an exercise to learn rust and scratch some niche needs
I had dealing with chrome traces.

## List 
List all processes included in the trace including their PID, types, # of 
threads, and process name.

## Filter
Produce a new trace that only includes the given renderer processes in addition
to all other non-renderer processes.

This is particularly useful when one collects a trace with a Chromium browser
that hase many open tabs (Renderer processes) but only wants to include the
particular renderer for the given site.

# Build & Usage

Once you have installed rust toolchain and cargo then simply:

``` 
$ cargo run -- --help
```

