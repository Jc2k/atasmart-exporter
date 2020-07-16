# atasmart_exporter

There are quite a few prometheus exporters available for smartmon. They tend to work by shelling out to smartmonctl and trying to parse the output.

Back in 2008 Lennart Poettering [created libatasmart](http://0pointer.de/blog/projects/being-smart.html). It's explicitly not a replacement for smartmon - it was meant to tell you the broad health data that is common to most drives. It was written so a distro could warning you promptly via the GUI when your hard drive was failing - maybe even warning you at install time as well. Since then Chris Holcombe has made a [rust wrapper](https://docs.rs/crate/libatasmart) for it. With the wonderful [prometheus_exporter](https://docs.rs/prometheus_exporter) crate a new exporter that didn't need to invoke a subprocess twice a minute almost wrote itself. 
