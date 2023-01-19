pub const BOOTSTRAP: [(&str, &str); 5] = [
    (
        "Flagellum",
        "
        (fabric
            (build
                (seed :left)
                (grow 30 (scale 90%))
            )
        )
        "
    ),
    (
        "Knee",
        "
        (fabric
            (build
                (branch
                    (face :A+ (grow 'XXX'))
                    (face :B+ (grow 3))
                )
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
                                (face :B-
                                    (grow 12 (scale 92%) (mark :halo-end))
                                )
                                (face :D-
                                    (grow 11 (scale 92%) (mark :halo-end))
                                )
                         )
                    )
              )
              (shape
                    (join :halo-end)
              )
        )
        "
    ),
    (
        "Zig Zag",
        "
        (fabric
            (build
                (branch
                    (face :C- (grow  3 (mark :end)))
                    (face :D- (grow 7
                        (face :B- (grow 7
                            (face :C- (grow 7
                                (face :D- (grow 7
                                    (face :C- (grow 7
                                        (face :D- (grow 3 (mark :end)))
                                    ))
                                ))
                            ))
                        ))
                    ))
                )
            )
            (shape
                (join :end)
            )
        )
        "
    ),
    (
        "Headless Hug",
        "
        (fabric
            (build
                (branch
                    (face :A+ (grow '....X..' (scale 95%) (mark :legs) ))
                    (face :B- (grow '....X..' (scale 95%) (mark :legs) ))
                    (face :A- (grow 3 (scale 90%)
                        (branch
                            (face :A+ (mark :shoulders))
                            (face :C+ (grow '...X..' (scale 93%) (mark :hands)))
                        )
                    ))
                    (face :B+ (grow 3 (scale 90%)
                        (branch
                            (face :A+ (mark :shoulders))
                            (face :C+ (grow '...X..' (scale 93%) (mark :hands)))
                        )
                    ))
                )
            )
            (shape
                (space :legs 5%)
                (space :hands 7%)
                (space :shoulders 5%)
            )
        )
        "
    ),
];
