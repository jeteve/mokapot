FROM mcr.microsoft.com/devcontainers/rust:latest

RUN apt-get update && apt-get install -y linux-perf && apt-get clean

# YaY https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=996839
RUN mkdir /usr/share/d3-flame-graph && \
    wget -O /usr/share/d3-flame-graph/d3-flamegraph-base.html https://cdn.jsdelivr.net/npm/d3-flame-graph@4/dist/templates/d3-flamegraph-base.html
