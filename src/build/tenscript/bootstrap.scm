;;;Seed
(fabric
  (build
    (seed :right-left)))
;;;Knee
(fabric
  (build
    (seed :right-left)
    (branch
      (face A+ (grow 3))
      (face B+ (grow 3))))
  (shape
    (finally :bow-tie-pulls)
    (finally :faces-to-triangles)))
;;;Flagellum
(fabric
  (build
    (seed :left)
    (grow 20 (scale .9))))
;;;Halo by Crane
(fabric
  (build
    (grow 4 (scale .92)
      (branch
        (face B- (grow 12 (scale .92) (mark :halo-end)))
        (face D- (grow 11 (scale .92) (mark :halo-end))))))
  (shape
    (join :halo-end)
    (finally :bow-tie-pulls)
    (finally :faces-to-triangles)))
;;;Headless Hug
(fabric
  (build
    (seed :right-left
      (orient-down A+ B+))
    (branch
      (face A- (grow "....X.." (scale .95) (mark :legs)))
      (face B+ (grow "....X.." (scale .95) (mark :legs)))
      (face A+
        (grow 3 (scale .9)
          (branch
            (face C+ (mark :shoulders))
            (face B+ (grow "...X.." (scale .93) (mark :hands))))))
      (face B-
        (grow 3 (scale .9)
          (branch
            (face D+ (mark :shoulders))
            (face C+ (grow "...X.." (scale .93) (mark :hands))))))))
  (shape
    (space :legs .05)
    (space :hands .07)
    (space :shoulders .05)
    (finally :bow-tie-pulls)
    (finally :faces-to-triangles)))

