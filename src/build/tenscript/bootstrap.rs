pub const BOOTSTRAP: [(&str, &str); 5] = [
    (
        "Flagellum",
        "
        (fabric
            (build
                (seed :left)
                (grow A- 30 (scale 90%))
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
                (grow :A+ 3)
                (grow :B+ 3))
            )
        )
        "
    ),
    (
        "Halo by Crane",
        "
        (fabric
              (build
                    (grow 5 (scale 92%)
                        (branch
                                (face B-
                                    (grow 12 (scale 92%) (mark :halo-end))
                                )
                                (face D-
                                    (grow 11 (scale 92%) (mark :halo-end))
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
            (build
                (seed :left)
                (branch
                    (grow C- 3
                        (mark A+ :end)
                    )
                    (grow D- 7
                        (grow B- 7
                            (grow C- 7
                                (grow D- 7
                                    (grow C- 7
                                        (grow D- 3 (mark A+ :end))
                                    )
                                )
                            )
                        )
                    )
                )
            )
            (shape
                (pull-together :end)
            )
        )
        "
    ),
    (
        "Headless Hug",
        "
        (fabric
            (build
                (seed :left)
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
];
