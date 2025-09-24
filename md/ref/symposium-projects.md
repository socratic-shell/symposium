# Symposium projects

A Symposium project is a host for taskspaces. It is configured to attach to a git repository.

## Local projects

Local projects are stored as a directory with a `.symposium` name. They contain a `.git` directory storing a clone and a set of `task-$UUID` directories, each of which is a taskspace. There are also some JSON configuration files.

## Remote projects

We would like to support remote projects (e.g., via ssh) but do not yet.

