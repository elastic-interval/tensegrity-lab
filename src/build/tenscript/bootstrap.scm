(fabrics
  (fabric
    (name "Seed")
    (build
      (seed :left)))
  (fabric
    (name "Knee")
    (build
      (seed :right-left)
      (branch
        (face A+ (grow 3))
        (face B+ (grow 3))))
    (shape
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Flagellum")
    (build
      (seed :left)
      (grow 20 (scale .9))))
  (fabric
    (name "Halo by Crane")
    (build
      (grow 4 (scale .92)
        (branch
          (face B- (grow 12 (scale .92) (mark :halo-end)))
          (face D- (grow 11 (scale .92) (mark :halo-end))))))
    (shape
      (join :halo-end)
      (remove-shapers) ; should automatically happen before vulcanize
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Headless Hug")
    (build
      (seed :right-left
        (orient-down A- B+))
      (branch
        (face A- (grow "....X.." (scale .95) (mark :legs)))
        (face B+ (grow "....X.." (scale .95) (mark :legs)))
        (face A+
          (grow 2 (scale .9)
            (branch
              (face C+ (mark :shoulders))
              (face D+ (grow "....X..." (scale .93) (mark :hands)))
              )))
        (face B-
          (grow 2 (scale .9)
            (branch
              (face D+ (mark :shoulders))
              (face C+ (grow "....X..." (scale .93) (mark :hands)))
              )))))
    (shape
      (countdown 25000
        (space :legs .5)
        (space :hands .01)
        (space :shoulders .05))
      (countdown 10000 (vulcanize))
      (remove-shapers)
      (replace-faces))))

