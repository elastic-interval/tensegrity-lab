"""
A tensegrity-lab fabric in RFEM 6, from its JSON
================================================

    python build_rfem.py HeadlessHug-2026-11-11.json --dry-run
    python build_rfem.py HeadlessHug-2026-11-11.json

--dry-run reads the JSON, makes the model and prints what it contains and
what it weighs, without RFEM. Without it, the model is built in the running
RFEM 6 through Dlubal's Python client, dlubal.api (RFEM 6.08 or later).


What the JSON gives
-------------------

The figure standing, as the simulation settles it: pretensioned, under
gravity, on its two feet and four guy lines. Every member comes with its
length here, its rest length (unloaded, what it is made to) and the force
it carries. Metres, kilograms, newtons, Z up, the ground at Z = 0. The
JSON says what is built; every choice about how to model it is made in this
file, and they are listed here.


How it is modelled
------------------

Strut      Three beams, inner 33x3 / outer 40x3 / inner 33x3 aluminium, split
           where the outer tube ends at its pin setting. The inner tubes'
           overlap inside the outer, and the pins' play, are left out.

Rings      Up each strut's bolt, a chain of rigid members from the end of the
           tubes through the centre of each ring, in slot order; from each ring
           a rigid member out to the pin of each cable it carries (about 30 mm).
           This is how Open Claw's model had them. A ring turns freely on its
           bolt, so it carries no torsion about the strut, which a rigid member
           does. The chain up the bolt is where an M12 beam could go instead.

Cables     Cable members, 6 mm 7x19, pin to pin. Fork rods (M8) are tension
           members, pin to pin.

Plates     Three rigid members between the plate's holes.

Hinges     A rigid fitting from each strut's stack to the shared pin; the eye's
           fitting is released in every rotation at the pin. Both struts spin
           freely on their bolts, so the hinge turns like a ball joint.

Feet       A rigid fitting (the rod end) from the stack to the ball, held there
           in translation only.

Guy lines  Cables from a strut's stack to an anchor on the ground, held fixed.


Pretension
----------

The structure only stands because it is pretensioned, so the pretension goes
in with the geometry. Each member is given the strain that takes it from its
length here back to its rest length:

    imposed strain = rest length / length - 1

Unloaded, every member would shrink or grow to its rest length; held in this
shape, a cable pulls and a strut pushes with about the force the simulation
found. PRESTRESS scales that: 1 is the simulation's pretension, 0 none.


Weight
------

From the JSON, not from RFEM's sections: each strut's tubes, the cables, and
the hardware at the nodes (caps, terminals, rings, plates, hinges). RFEM's
self-weight is switched off, so the model weighs what the simulation weighed,
to within a couple of kilograms (the simulation measures cables joint to
joint, and hangs terminals on the guy lines' ends); the dry run prints both.


Analysis
--------

One load case, "Standing": the pretension and the weight together, analysed
with large deformations, since a tensegrity has no stiffness without its
pretension. If RFEM and the simulation agree, the figure stays where it is,
moving only by millimetres. Where it moves more, the two models differ: in the
tube sections, the ring offsets, or the ground.


Where this is a guess
---------------------

The lines marked CHECK use names from dlubal.api that have not yet been run
against RFEM: member types, the hinge release, supports, the analysis
setting and the member and nodal loads. RFEM_NAMES gives the library names to
look up. Each group of objects is sent to RFEM separately, so one that fails
says which, and the rest still arrive.
"""

import argparse
import json

# ---------------------------------------------------------------------------
# Settings: change these
# ---------------------------------------------------------------------------

PRESTRESS = 1.0

# Names in RFEM's libraries, for the JSON's materials, tubes and cable stock.
RFEM_NAMES = {
    "aluminium": "EN AW-6060 T66",
    "rope": "Wire rope (user-defined)",
    "steel": "S235",
    "outer": "CHS 40x3",
    "inner": "CHS 33x3",
    "cable": "R 6 (user-defined, area from the JSON)",
    "rod": "M8 (user-defined, stress area from the JSON)",
}

GRAVITY = 9.81
SCHEMA_VERSION = 2


# ---------------------------------------------------------------------------
# The model, in plain Python
# ---------------------------------------------------------------------------

class Member:
    def __init__(self, name, kind, start, end, section=None, strain=None, weight=None):
        self.name = name
        self.kind = kind          # beam, rigid, cable, rod or guy
        self.start = start        # node names
        self.end = end
        self.section = section    # a tube or cable stock name, None for rigid
        self.strain = strain      # imposed strain for the pretension, or None
        self.weight = weight      # its own weight in N/m, or None
        self.released = False     # every rotation free at its end node


class Model:
    def __init__(self):
        self.nodes = {}           # name -> (x, y, z)
        self.members = []
        self.supports = []        # (node, "pinned" or "fixed")
        self.node_weights = []    # (node, N, what)

    def node(self, name, position):
        if name in self.nodes:
            placed = self.nodes[name]
            apart = max(abs(a - b) for a, b in zip(placed, position))
            assert apart < 1e-4, f"node {name} placed twice, {apart * 1000:.1f} mm apart"
        else:
            self.nodes[name] = tuple(position)
        return name

    def add(self, member):
        assert all(m.name != member.name for m in self.members), f"member {member.name} twice"
        self.members.append(member)
        return member

    def length(self, member):
        a, b = self.nodes[member.start], self.nodes[member.end]
        return sum((p - q) ** 2 for p, q in zip(a, b)) ** 0.5


def prestrain(rest_length, length):
    return PRESTRESS * (rest_length / length - 1)


def stack_node(strut, end):
    """A strut end's stack: the joint itself, or `<pin>@<far joint>` where a
    hinge or foot fitting sets the stack back from its pin."""
    if end["closed_by"] == "washer and lock nut":
        return end["joint"]
    far = [e for e in strut["ends"] if e is not end][0]
    return f"{end['joint']}@{far['joint']}"


def strut_end(strut, joint):
    return [end for end in strut["ends"] if end["joint"] == joint][0]


def add_struts(model, data):
    for strut in data["struts"]:
        name = strut["name"]
        a, b = strut["ends"]
        start = model.node(stack_node(strut, a), a["stack"])
        end = model.node(stack_node(strut, b), b["stack"])
        model.node_weights.append((start, a["hardware_mass"] * GRAVITY, "strut cap, bolt, nut, washers"))
        model.node_weights.append((end, b["hardware_mass"] * GRAVITY, "strut cap, bolt, nut, washers"))
        inner1, outer, inner2 = strut["segments"]
        first = model.node(f"{name}.1", outer["from"])
        second = model.node(f"{name}.2", outer["to"])
        strain = prestrain(strut["rest_length"], strut["length"])
        weight = strut["tube_mass"] / strut["length"] * GRAVITY
        model.add(Member(f"{name}.inner1", "beam", start, first, "inner", strain, weight))
        model.add(Member(f"{name}.outer", "beam", first, second, "outer", strain, weight))
        model.add(Member(f"{name}.inner2", "beam", second, end, "inner", strain, weight))


def add_hinges_and_feet(model, data, struts):
    for hinge in data["hinges"]:
        pin = model.node(hinge["pin"], hinge["position"])
        model.node_weights.append((pin, hinge["mass"] * GRAVITY, "hinge clevis, pin and eye"))
        for role in ("clevis", "eye"):
            strut = struts[hinge[role + "_strut"]]
            stack = stack_node(strut, strut_end(strut, hinge["pin"]))
            fitting = model.add(Member(f"{stack}:fitting", "rigid", stack, pin))
            fitting.released = role == "eye"
    for foot in data["feet"]:
        strut = struts[foot["strut"]]
        stack = stack_node(strut, strut_end(strut, foot["joint"]))
        ball = model.node(foot["joint"], foot["position"])
        model.add(Member(f"{stack}:fitting", "rigid", stack, ball))
        model.supports.append((ball, "pinned"))


def add_plates(model, data):
    for plate in data["plates"]:
        holes = [model.node(f"{plate['name']}.{i + 1}", hole) for i, hole in enumerate(plate["holes"])]
        for hole in holes:
            model.node_weights.append((hole, plate["mass"] / 3 * GRAVITY, "a third of the node plate"))
        for i, (a, b) in enumerate([(0, 1), (1, 2), (2, 0)]):
            model.add(Member(f"{plate['name']}.edge{i + 1}", "rigid", holes[a], holes[b]))


def ring_node(model, stack, end):
    return model.node(f"{stack}#{end['slot']}", end["ring_center"])


def add_rings(model, data, struts):
    """The chain of rigid members up each bolt, through the rings in slot order."""
    rings = {}
    for tension in data["tension"]:
        for end in tension["ends"]:
            if end["on"] == "stack":
                strut = struts[end["strut"]]
                stack = stack_node(strut, strut_end(strut, end["joint"]))
                rings.setdefault(stack, {})[end["slot"]] = ring_node(model, stack, end)
    for stack, slots in rings.items():
        below = stack
        for slot in sorted(slots):
            model.add(Member(f"{stack}:axial{slot}", "rigid", below, slots[slot]))
            below = slots[slot]


def cable_end_node(model, end, far, struts):
    """Where a cable or rod is pinned: a pin out from its ring, or a plate hole."""
    if end["on"] == "plate":
        hole = model.node(f"{end['plate']}.{end['hole']}", end["position"])
        model.node_weights.append((hole, end["hardware_mass"] * GRAVITY, "terminal or rod fork"))
        return hole
    strut = struts[end["strut"]]
    stack = stack_node(strut, strut_end(strut, end["joint"]))
    pin = model.node(f"{stack}/{far}", end["pivot"])
    model.add(Member(f"{pin}:radial", "rigid", ring_node(model, stack, end), pin))
    model.node_weights.append((pin, end["hardware_mass"] * GRAVITY, "terminal or rod fork, ring, washer"))
    return pin


def where(end):
    return end["joint"] if end["on"] == "stack" else end["plate"]


def add_tension(model, data, struts):
    stock = {s["name"]: s for s in data["tension_stock"]}
    for tension in data["tension"]:
        a, b = tension["ends"]
        start = cable_end_node(model, a, where(b), struts)
        end = cable_end_node(model, b, where(a), struts)
        kind = "cable" if tension["stock"] == "cable" else "rod"
        strain = prestrain(tension["rest_pin_to_pin"], tension["pin_to_pin"])
        weight = stock[tension["stock"]]["mass_per_metre"] * GRAVITY
        model.add(Member(tension["name"], kind, start, end, tension["stock"], strain, weight))


def add_guy_lines(model, data, struts):
    cable = [s for s in data["tension_stock"] if s["name"] == "cable"][0]
    for guy in data["guy_lines"]:
        strut = struts[guy["strut"]]
        stack = stack_node(strut, strut_end(strut, guy["joint"]))
        anchor = model.node(guy["anchor"], guy["anchor_position"])
        model.supports.append((anchor, "fixed"))
        strain = prestrain(guy["rest_length"], guy["length"])
        weight = cable["mass_per_metre"] * GRAVITY
        model.add(Member(f"{guy['joint']}-{guy['anchor']}", "guy", stack, anchor, "cable", strain, weight))


def make_model(data):
    if data.get("schema_version") != SCHEMA_VERSION:
        raise SystemExit(f"This script reads schema {SCHEMA_VERSION}; the JSON is {data.get('schema_version')}.")
    struts = {strut["name"]: strut for strut in data["struts"]}
    model = Model()
    add_struts(model, data)
    add_hinges_and_feet(model, data, struts)
    add_plates(model, data)
    add_rings(model, data, struts)
    add_tension(model, data, struts)
    add_guy_lines(model, data, struts)
    return model


def print_summary(data, model):
    kinds = {}
    for member in model.members:
        kinds[member.kind] = kinds.get(member.kind, 0) + 1
    members = ", ".join(f"{n} {kind}" for kind, n in sorted(kinds.items()))
    weight = sum(m.weight * model.length(m) for m in model.members if m.weight)
    weight += sum(newtons for _, newtons, _ in model.node_weights)
    strut_forces = sorted(s["force"] for s in data["struts"])
    pulls = sorted(t["force"] for t in data["tension"] + data["guy_lines"])
    print(f"{data['fabric']}, {data['moment']}, created {data['created']}")
    print(f"  {len(model.nodes)} nodes, {len(model.members)} members: {members}")
    print(f"  {len(model.supports)} supports, {sum(m.released for m in model.members)} hinge releases")
    print(f"  weight {weight / GRAVITY:.1f} kg; the simulation weighed {data['total_mass']:.1f} kg")
    print(f"  prestress x{PRESTRESS}: the simulation's struts push {-strut_forces[-1] / 1e3:.1f} to "
          f"{-strut_forces[0] / 1e3:.1f} kN, cables pull up to {pulls[-1] / 1e3:.1f} kN")


# ---------------------------------------------------------------------------
# Sending it to RFEM
# ---------------------------------------------------------------------------

def build_in_rfem(data, model):
    from dlubal.api import rfem

    core = rfem.structure_core
    inf = float("inf")
    node_no = {name: no for no, name in enumerate(model.nodes, 1)}
    member_no = {m.name: no for no, m in enumerate(model.members, 1)}
    materials = [m["name"] for m in data["materials"]]
    sections = {t["name"]: t["material"] for t in data["tubes"]}
    sections.update({s["name"]: s["material"] for s in data["tension_stock"]})
    section_no = {name: no for no, name in enumerate(sections, 1)}

    # CHECK: member type names.
    member_types = {
        "beam": core.Member.TYPE_BEAM,
        "rigid": core.Member.TYPE_RIGID,
        "cable": core.Member.TYPE_CABLE,
        "rod": core.Member.TYPE_TENSION,
        "guy": core.Member.TYPE_CABLE,
    }

    groups = {}
    groups["materials"] = [
        core.Material(no=no, name=RFEM_NAMES[name]) for no, name in enumerate(materials, 1)
    ]
    groups["sections"] = [
        core.CrossSection(no=section_no[name], name=RFEM_NAMES[name], material=materials.index(material) + 1)
        for name, material in sections.items()
    ]
    groups["nodes"] = [
        core.Node(no=node_no[name], coordinate_1=x, coordinate_2=y, coordinate_3=z, comment=name)
        for name, (x, y, z) in model.nodes.items()
    ]
    # CHECK: a hinge's springs, 0 for free.
    groups["hinges"] = [
        rfem.types_for_members.MemberHinge(no=1, moment_release_mt=0.0, moment_release_my=0.0, moment_release_mz=0.0)
    ]
    groups["lines"] = [
        core.Line(no=member_no[m.name], definition_nodes=[node_no[m.start], node_no[m.end]]) for m in model.members
    ]
    members = []
    for m in model.members:
        member = core.Member(no=member_no[m.name], line=member_no[m.name], type=member_types[m.kind], comment=m.name)
        if m.section:
            member.cross_section_start = section_no[m.section]
        if m.released:
            member.member_hinge_end = 1
        members.append(member)
    groups["members"] = members

    # CHECK: support field names.
    supports = []
    for no, (node, how) in enumerate(model.supports, 1):
        support = rfem.types_for_nodes.NodalSupport(no=no, nodes=[node_no[node]], spring_x=inf, spring_y=inf, spring_z=inf)
        if how == "fixed":
            support.rotational_restraint_x = inf
            support.rotational_restraint_y = inf
            support.rotational_restraint_z = inf
        supports.append(support)
    groups["supports"] = supports

    # CHECK: analysis setting, load case and load field names. Loads act down (-Z), in N and N/m.
    loads = [
        rfem.loading.StaticAnalysisSettings(
            no=1, analysis_type=rfem.loading.StaticAnalysisSettings.ANALYSIS_TYPE_LARGE_DEFORMATIONS),
        rfem.loading.LoadCase(no=1, name="Standing", static_analysis_settings=1, self_weight_active=False),
    ]
    no = 0
    for m in model.members:
        if m.strain:
            no += 1
            loads.append(rfem.loads.MemberLoad(
                no=no, load_case=1, members=[member_no[m.name]],
                load_type=rfem.loads.MemberLoad.LOAD_TYPE_AXIAL_STRAIN, magnitude=m.strain))
        if m.weight:
            no += 1
            loads.append(rfem.loads.MemberLoad(
                no=no, load_case=1, members=[member_no[m.name]],
                load_type=rfem.loads.MemberLoad.LOAD_TYPE_FORCE,
                load_direction=rfem.loads.MemberLoad.LOAD_DIRECTION_GLOBAL_Z_OR_USER_DEFINED_W_TRUE,
                magnitude=m.weight))
    for no, (node, newtons, _) in enumerate(model.node_weights, 1):
        loads.append(rfem.loads.NodalLoad(
            no=no, load_case=1, nodes=[node_no[node]],
            load_direction=rfem.loads.NodalLoad.LOAD_DIRECTION_GLOBAL_Z_OR_USER_DEFINED_W_TRUE,
            force_magnitude=newtons))
    groups["loads"] = loads

    with rfem.Application() as app:
        app.create_model(name=data["fabric"])
        app.delete_all_objects()
        for group, objects in groups.items():
            try:
                app.create_object_list(objects)
                print(f"{group}: {len(objects)}")
            except Exception as error:
                print(f"{group}: FAILED, {error}")


def main():
    parser = argparse.ArgumentParser(description="Build a tensegrity-lab fabric in RFEM 6 from its JSON.")
    parser.add_argument("json", help="the fabric's JSON from tensegrity-lab")
    parser.add_argument("--dry-run", action="store_true", help="make the model without RFEM and summarise it")
    args = parser.parse_args()
    with open(args.json) as file:
        data = json.load(file)
    model = make_model(data)
    print_summary(data, model)
    if not args.dry_run:
        build_in_rfem(data, model)


if __name__ == "__main__":
    main()
