(fabric-library
  (fabric (name "Seed" "Single") (build (branch (alias Single))))
  (fabric (name "Seed" "Omni") (build (branch (alias Omni))))
  (fabric (name "Seed" "Torque") (build (branch (alias Torque))))
  (fabric
    (name "Simple" "Knee")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 3))
        (face (alias Front:Left) (grow 3))))
    (shape
      (vulcanize)
      (faces-to-triangles)))
  (fabric
    (name "Simple" "Flagellum")
    (build
      (branch (alias Single)
        (face (alias :next-base) (grow 20 (scale .9)))))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :frozen)))
  (fabric
    (name "Simple" "Tetrapod")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 3 (scale .9)))
        (face (alias Bottom:Right) (grow 3 (scale .9)))
        (face (alias Back:Left) (grow 3 (scale .9)))
        (face (alias Front:Left) (grow 3 (scale .9)))
        ))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Simple" "Twist8")
    (build
      (branch (alias Omni)
        (face (alias Top:Right) (grow 12 (scale .92) (mark :ring-a)))
        (face (alias Bottom:Right) (grow 13 (scale .92) (mark :ring-a)))
        (face (alias Back:Left) (grow 12 (scale .92) (mark :ring-b)))
        (face (alias Front:Left) (grow 13 (scale .92) (mark :ring-b)))
        ))
    (shape
      (during 15000 (join :ring-a) (join :ring-b))
      (during 20000 (vulcanize))
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Simple" "Bulge Ring")
    (build
      (branch (alias Single)
        (face (alias :base) (grow 8 (scale .92) (mark :tip)))
        (face (alias :next-base) (grow 9 (scale .92) (mark :tip)))))
    (shape (join :tip))
    (pretense (surface :bouncy)))
  (fabric
    (name "Art" "Halo by Crane")
    (build
      (branch (alias Single) (rotate) (rotate)
        (face (alias :next-base)
          (grow 4 (scale .92)
            (branch (alias Omni)
              (face (alias TopX) (grow 12 (scale .92) (mark :halo-end)))
              (face (alias TopY) (grow 11 (scale .92) (mark :halo-end))))))))
    (shape
      (join :halo-end)
      (during 30000 (remove-spacers)) ; TODO: should automatically happen before vulcanize
      (vulcanize)
      (faces-to-triangles)))
  (fabric
    (name "Art" "K-10")
    (build
      (branch (alias Torque)
        (face (alias Left:Front:Bottom)
          (branch (alias Torque)
            (face (alias Far:Front) (grow 3))))
        (face (alias Left:Back:Bottom)
          (branch (alias Torque)
            (face (alias Far:Front) (grow 3))))
        (face (alias Right:Front:Bottom)
          (grow 1
            (branch (alias Torque) (rotate)
              (face (alias Far:Base) (grow 2)))))
        (face (alias Right:Back:Bottom)
          (grow 1
            (branch (alias Torque) (rotate)
              (face (alias Far:Base) (grow 2)))))))
    (shape
      (vulcanize)
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  (fabric
    (name "Art" "Tommy Torque")
    (build
      (branch (alias Omni)
        (face (alias Top:Left)
          (grow 1
            (branch (alias Torque)
              (face (alias Far:Front) (grow 3 (scale .85))))))
        (face (alias Top:Right)
          (grow 1
            (branch (alias Torque) (rotate)
              (face (alias Far:Front) (grow 3 (scale .85))))))
        (face (alias Back:Right)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias Far:Back)
              (branch (alias Torque) (scale .5) (rotate)
                (face (alias Far:Side) (grow 1 (scale .5)))
                (face (alias Far:Back) (grow 1 (scale .5)))
                (face (alias Far:Front) (mark :sole))
                (face (alias Far:Base) (mark :sole))))))
        (face (alias Back:Left)
          (branch (alias Torque)
            (face (alias Far:Back)
              (branch (alias Torque) (scale .5) (rotate)
                (face (alias Far:Side) (grow 1 (scale .5)))
                (face (alias Far:Back) (grow 1 (scale .5)))
                (face (alias Far:Front) (mark :sole))
                (face (alias Far:Base) (mark :sole))))))))
    (shape
      (down :sole)
      (faces-to-triangles))
    (pretense
      (surface :frozen)
      (muscle 0.1)
      ))
  (fabric
    (name "Art" "Headless Hug")
    (build
      (branch (alias Omni)
        (face (alias Bottom:Left) (grow "....X.." (scale .95) (mark :legs)))
        (face (alias Bottom:Right) (grow "....X.." (scale .95) (mark :legs)))
        (face (alias Top:Left)
          (grow 2 (scale .9)
            (branch (alias Omni)
              (face (alias TopZ) (mark :chest-1))
              (face (alias BotX) (mark :chest-2))
              (face (alias BotY) (grow "....X..." (scale .93) (mark :hands))))))
        (face (alias Top:Right)
          (grow 2 (scale .9)
            (branch (alias Omni)
              (face (alias TopY) (mark :chest-1))
              (face (alias BotZ) (mark :chest-2))
              (face (alias BotX) (grow "....X..." (scale .93) (mark :hands))))))))
    (shape
      (during 15000 (space :legs .3) (space :hands .3) (space :chest-1 .8) (space :chest-2 .2))
      (during 80000 (vulcanize))
      (remove-spacers)
      (faces-to-triangles)))
  (fabric (name "Art" "Torque Island")
    (build
      (branch (alias Torque)
        (face (alias Left:Front:Bottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias Far:Side) (mark :loose))))
        (face (alias Right:Front:Bottom)
          (branch (alias Torque) (rotate) (rotate)
            (face (alias Far:Side)
              (branch (alias Torque)
                (face (alias Far:Side) (mark :loose))))))))
    (shape
      (join :loose)
      (faces-to-triangles))
    (pretense (surface :bouncy)))
  )