# Elastic Interval Geometry

There are many kinds of geometry, each with their own base assumptions. EIG assumes that there are only intervals spanning a preferred distance between pairs of joints.

Structures are configurations of intervals with their ends attached to each other at joints, and they exist in time so they are four dimensional.

Time is part of the model much like how it works in [cellular automata](https://en.wikipedia.org/wiki/Cellular_automaton), where all elements experience a moment of time simultaneously and their mutual influences are local.

### Interval

An interval expresses a preferred span between two joints by acting on them.

A day in the life of an interval:

1. wake up
2. check current span
3. more than ideal => pull joints together
4. less than ideal => push joints apart
5. go to sleep

The interval only knows about its two joints, and pushing/pulling a joint amounts to contributing to a force vector located there, which the joints remember.

### Joint

A joint is a place where multiple intervals come together and it is considered to have insignificant inherent mass.  Its effective mass is contributed by the masses of the intervals it holds together.

A day in the life of a joint:

1. wake up
2. add up pushes and pulls
3. increase velocity in the direction of the sum
4. jump in space to a new location based on velocity
5. go to sleep

### Time

The passage of time is defined as sweeping computations or "ticks of the clock" which first wakes all of the intervals, and then wakes all of the joints. The jumps taken by the joints change the spans that the intervals see.

### Gravity

Of course gravity is just an acceleration downwards applied to each joint during each tick of the clock, but gravity means nothing without a surface to oppose it, since falling is the same as weightlessness.

The surface introduces a whole new and interesting problem, because it represents a second sense of locality beyond the locality represented by an interval's effects on its two joints.  

Now a joint will have to react differently when it finds itself under the surface, pushing back up against gravity somehow. This provides an interesting challenge, because we now have to determine whether a joint's reaction to the surface is related to *how far under* it finds itself. If so, then we have to ask why it is this far under, since it may have jumped from nearer or farther from the surface.  Something to think about.


