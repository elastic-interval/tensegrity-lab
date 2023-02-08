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
        (face F1 (grow 3))
        (face F3 (grow 3))))
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
          (face F2 (grow 12 (scale .92) (mark :halo-end)))
          (face F6 (grow 11 (scale .92) (mark :halo-end))))))
    (shape
      (join :halo-end)
      (remove-shapers) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (replace-faces)))
  (fabric
    (name "Headless Hug")
    (build
      (seed :right-left
        (orient-down F0 F3))
      (branch
        (face F0 (grow "....X.." (scale .95) (mark :legs)))
        (face F3 (grow "....X.." (scale .95) (mark :legs)))
        (face F1
          (grow 2 (scale .9)
            (branch
              (face F5 (mark :shoulders))
              (face F7 (grow "....X..." (scale .93) (mark :hands)))
              )))
        (face F2
          (grow 2 (scale .9)
            (branch
              (face F7 (mark :shoulders))
              (face F5 (grow "....X..." (scale .93) (mark :hands)))
              )))))
    (shape
      (countdown 25000
        (space :legs .5)
        (space :hands .01)
        (space :shoulders .05))
      (countdown 10000 (vulcanize))
      (remove-shapers)
      (replace-faces))))

