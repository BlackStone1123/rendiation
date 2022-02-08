use crate::*;

use crate as shadergraph;

glsl_function!(
  "
    vec3 importanceSampleCone(const in vec2 coords, const in float cosConeAngle) {
      float phi = TWO_PI * coords.x;
    
      float cosTheta = cosConeAngle * (1.0 - coords.y) + coords.y;
      float sinTheta = sqrt(max(0.0001, 1.0 - cosTheta * cosTheta));
    
      vec3 h;
      h.x = sinTheta * cos(phi);
      h.y = sinTheta * sin(phi);
      h.z = cosTheta;
    
      return h;
    }
  "
);

glsl_function!(
  "
  float linstep(float minVal, float maxVal, float val) {
    // NOTE: smoothstep is important to the seams introduced by the cubemap
    return smoothstep(minVal, maxVal, val);
  }
  "
);

glsl_function!(
  "
  float reduceLightBleeding(float p_max, float amount) {
    return linstep(amount, 1.0, p_max);
    }
  "
);

glsl_function!(
    "
  float chebyshevUpperBound(vec2 moments, float mean, float minVariance, float lightBleedingReduction)
  {
  // Compute variance
  float variance = moments.y - (moments.x * moments.x);
  variance = max(variance, minVariance);
  
  // Compute probabilistic upper bound
  float d = mean - moments.x;
  float pMax = variance / (variance + (d * d));
  
  pMax = reduceLightBleeding(pMax, lightBleedingReduction);
  
  // One-tailed Chebyshev
  return (mean <= moments.x ? 1.0 : pMax);
  }
  "
  );

#[test]
fn build_shader_function() {
  let a = consts(1).mutable();
  let c = consts(0).mutable();

  for_by(5, |for_ctx, i| {
    let b = 1;
    if_by(i.greater_than(0), || {
      a.set(a.get() + b.into());
      for_ctx.do_continue();
    });
    c.set(c.get() + i);
  });

  // let d = my_shader_function(1.2, 2.3);
}

// #[shader_function]
// pub fn my_shader_function(a: Node<f32>, b: Node<f32>) -> Node<f32> {
//     let c = a + b;
//     if_by(c.greater_than(0.), || early_return(2.));
//     c + 1.0.into()
// }

// pub fn my_shader_function(a: impl Into<Node<f32>>, b: impl Into<Node<f32>>) -> Node<f32> {
//   let a = a.into();
//   let b = b.into();

//   function((a, b), |(a, b)| {
//     let c = a + b;
//     if_by(c.greater_than(0.), || early_return(2.));
//     c + 1.0.into()
//   })
// }

struct Test;

impl ShaderGraphProvider for Test {
  fn build_vertex(
    &self,
    builder: &mut ShaderGraphVertexBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    let a = consts(1.) + consts(2.);
    let a: Node<_> = (Vec3::zero(), a).into();
    builder.vertex_position.set(a);

    builder.vertex_position.set(Vec4::zero());

    let a = consts(1.).mutable();
    let c = consts(0.).mutable();

    for_by(5, |for_ctx, i| {
      let b = 1.;
      if_by(i.greater_than(0), || {
        a.set(a.get() + b.into());
        for_ctx.do_continue();
      });

      let r: Node<Vec4<f32>> = (Vec3::zero(), a.get()).into();
      builder.vertex_position.set(r);
    });

    if_by(false, || {
      a.set(a.get() + c.get());
      let r: Node<Vec4<f32>> = (Vec3::zero(), a.get()).into();
      builder.vertex_position.set(r);
    });

    let x = reduceLightBleeding(a.get(), 2.);
    builder.vertex_point_size.set(x);

    Ok(())
  }

  fn build_fragment(
    &self,
    _builder: &mut ShaderGraphFragmentBuilder,
  ) -> Result<(), ShaderGraphBuildError> {
    // default do nothing
    Ok(())
  }
}

#[test]
fn test_build_shader() {
  let result = build_shader(&Test, &WGSL).unwrap();

  println!("vertex: \n{}", result.vertex_shader);
  println!("fragment: \n{}", result.frag_shader);
}
