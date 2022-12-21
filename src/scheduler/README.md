## djtool task scheduler

_Note_: If this implementation turns out to be well designed and useful for different situations, it will become a stand alone external dependency for djtool.

#### Goals of this implementation

There exist many different task scheduler implementations, the goals of this implementation are as follows:

- support async tasks
- support directed acyclic graphs with cycle detection
- allow custom scheduling policies based on a custom ID type, which could enforce complex constraints by using labels
- allow concurrent adding of tasks while the scheduler is executing, so that tasks can be streamed into the scheduler

#### TODO

- implement pause / resume using notfications
- implement notification based waiting for ready tasks
- use select instead of shutdown task

#### Usage

tba
