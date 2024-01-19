
<p align="center">
  <img src="./asset/rrf.svg" alt="rrf logo" style="margin: auto">
</p>

# Rendiation Rendering Framework

RRF is a structured and comprehensive graphics framework for versatile interactive visualization.

The center of the framework consists of a production ready scene graph and a shader EDSL implementation. Several innovative rendering engine architecture design ideas are explored and applied. For example, the composability of the complex effects or optimization behaviors, the parallel reactive incremental systems, and the extensibility of the scene content representation.

Many handcrafted libraries of basic concepts in the graphics realm support the above center crates. Data structures, algorithms, and common operations in different graphics domains like mesh, texture, lighting, animation, and space partitions. Under these crates, there are foundational supports like mathematics, reactive primitives, generic data containers, and platform graphics API abstractions(here we directly embrace and encapsulate wgpu).

Leveraging these crates, users could build or even assemble and customize their own featured high-performance viewers, offline data assets pipelines, or any other highly demanded graphics-related tasks in a well-organized way with ease.

### Development

Install the default rust toolchain and everything should works fine.

RRF uses language features such as const generics and specialization, the nightly compiler is required. The cargo will automatically switch to the correct nightly version.
