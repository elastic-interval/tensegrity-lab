;;;Seed
(fabric
  (build
    (seed :right-left)))
;;;Flagellum
(fabric
  (build
    (seed :left)
    (grow 30 (scale .9))))
;;;Knee
(fabric
  (build
    (branch
      (face :A+ (grow "XXX"))
      (face :B+ (grow 3)))
    )
  )
;;;Halo by Crane
(fabric
  (build
    (grow 4 (scale .92)
      (branch
        (face :B- (grow 12 (scale .92) (mark :halo-end)))
        (face :D- (grow 11 (scale .92) (mark :halo-end))))
      )
    )
  (shape
    (join :halo-end)
    (finally :bow-tie-pulls)
    (finally :faces-to-triangles)
    ))
;;;Zig Zag
(fabric
  (build
    (branch
      (face :C- (grow 3 (mark :end)))
      (face :D-
        (grow 7
          (face :B-
            (grow 7
              (face :C-
                (grow 7
                  (face :D-
                    (grow 7
                      (face :C-
                        (grow 7
                          (face :D- (grow 3 (mark :end))))))))))))))
    )
  (shape (join :end)))
;;;Headless Hug
(fabric
  (build
    (seed :right-left)
    (branch
      (face :A- (grow "....X.." (scale .95) (mark :legs)))
      (face :B+ (grow "....X.." (scale .95) (mark :legs)))
      (face :A+
        (grow 3 (scale .9)
          (branch
            (face :C+ (mark :shoulders))
            (face :B+ (grow "...X.." (scale .93) (mark :hands)))
            )
          ))
      (face :B-
        (grow 3 (scale .9)
          (branch
            (face :D+ (mark :shoulders))
            (face :C+ (grow "...X.." (scale .93) (mark :hands)))
            )
          ))
      )
    )
  (shape
    (space :legs .05)
    (space :hands .07)
    (space :shoulders .05)))

