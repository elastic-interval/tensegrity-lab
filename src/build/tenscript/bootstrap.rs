pub const BOOTSTRAP: [(&str, &str); 7] = [
    (
        "Flagellum",
        "
        (fabric
            (build
                (seed :left)
                (scale 90%)
                (grow A- 30)
            )
        )
        "
    ),
    (
        "Knee",
        "
        (fabric
            (surface :frozen)
            (build
            (branch
                (grow A 3)
                (grow b 3))
            )
        )
        "
    ),
    (
        "Halo by Crane",
        "
        (fabric
              (build
                    (seed :left)
                    (grow A+ 5 (scale 92%)
                        (branch
                                (grow B- 12 (scale 92%)
                                     (branch (mark A+ :halo-end))
                                )
                                (grow D- 11 (scale 92%)
                                    (branch (mark A+ :halo-end))
                                )
                         )
                    )
              )
              (shape
                (pull-together :halo-end)
              )
        )
        "
    ),
    (
        "Zig Zag",
        "
        (fabric
            (seed :left-right)
            (surface :frozen)
            (build
                (branch
                    (grow C- 3
                        (mark A+ :end)
                    )
                    (grow D- 7
                    (grow B- 7)
                    (grow C- 7)
                    (grow D- 7)
                    (grow C- 7)
                    (grow D- 3)
                    (mark A+ :end)
                )
            )
            (shape
                (pull-together :end)
            )
        )
        "
    ),
    (
        "Composed Tree",
        "
        (fabric
          (seed :left)
          (def (subtree scale-num)
            (branch
              (grow B- 5)
              (grow C- 5)
              (grow D- 5)))
          (branch
            (grow A+ 6)
            (grow b 4 (subtree))
            (grow c 4 (subtree))
            (grow d 4 (subtree)))
        )
        "
    ),
    (
        "Headless Hug",
        "
        (fabric
            (build
                (seed :left-right)
                (branch
                    (grow A+
                        (scale 95%)
                        (twist 0 0 0 0 1 0 0)
                        (mark A+ :legs)
                     )
                (grow B-
                    (scale 95%)
                    (twist 0 0 0 0 1 0 0)
                    (mark A+ :legs)
                )
                (grow A-
                    (scale 90%)
                    (branch
                        (grow A+ 3
                        (mark A+ :shoulders)
                    )
                    (grow C+
                        (scale 93%)
                        (twist 1 0 0 0 1 0 0)
                        (mark A+ :hands)
                    )
                )
                (grow B+
                    (scale 90%)
                    (branch
                        (grow A+ 3
                            (mark A+ :shoulders)
                        )
                        (grow C+
                            (scale 93%)
                            (twist 1 0 0 0 1 0 0)
                            (mark A+ :hands)
                        )
                    )
                )
            )
            (shape
                (pull-together :legs 5%)
                (pull-together :hands 7%)
                (pull-together :shoulders 5%)
            )
        )
        "
    ),
    (
        "",
        ""
    ),
];
