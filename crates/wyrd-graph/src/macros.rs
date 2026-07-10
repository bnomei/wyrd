/// Builds and validates a [`Weave`](crate::Weave) with the typed graph API.
///
/// The declaration order is `id`, optional `numeric`, `knots`, optional
/// `patterns`, then `threads`. Knot ports use identifier syntax (`source.out`),
/// while pattern exports use `out(expr)` and `in(expr)`.
///
/// The macro is deliberately only syntax sugar: expressions are evaluated once
/// in source order and all endpoint and graph checks are performed by
/// [`WeaveBuilder`](crate::WeaveBuilder).
///
/// ```
/// use wyrd_graph::{weave, BuildError, KnotKind, Weave};
///
/// fn graph() -> Result<Weave, BuildError> {
///     weave! {
///         id: "inverter";
///         knots {
///             source = KnotKind::signal_in();
///             invert = KnotKind::not();
///             sink as "debug.inverted" = KnotKind::signal_out("debug.inverted");
///         }
///         threads {
///             source.out -> invert.in;
///             invert.out -> sink.in;
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! weave {
    {
        id: $id:expr;
        numeric: $numeric:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        patterns {
            $( $pattern_binding:ident = ($instance_id:expr, $pattern:expr); )*
        }
        threads { $($threads:tt)* }
    } => {
        $crate::__weave_expand! {
            id: $id;
            numeric: [$numeric];
            knots { $( $knot $(as $alias)? = $kind; )* }
            patterns { $( $pattern_binding = ($instance_id, $pattern); )* }
            threads { $($threads)* }
        }
    };

    {
        id: $id:expr;
        numeric: $numeric:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        threads { $($threads:tt)* }
    } => {
        $crate::__weave_expand! {
            id: $id;
            numeric: [$numeric];
            knots { $( $knot $(as $alias)? = $kind; )* }
            patterns { }
            threads { $($threads)* }
        }
    };

    {
        id: $id:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        patterns {
            $( $pattern_binding:ident = ($instance_id:expr, $pattern:expr); )*
        }
        threads { $($threads:tt)* }
    } => {
        $crate::__weave_expand! {
            id: $id;
            numeric: [];
            knots { $( $knot $(as $alias)? = $kind; )* }
            patterns { $( $pattern_binding = ($instance_id, $pattern); )* }
            threads { $($threads)* }
        }
    };

    {
        id: $id:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        threads { $($threads:tt)* }
    } => {
        $crate::__weave_expand! {
            id: $id;
            numeric: [];
            knots { $( $knot $(as $alias)? = $kind; )* }
            patterns { }
            threads { $($threads)* }
        }
    };
}

/// Implementation detail for [`weave!`](crate::weave).
#[doc(hidden)]
#[macro_export]
macro_rules! __weave_expand {
    {
        id: $id:expr;
        numeric: [$($numeric:expr)?];
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        patterns {
            $( $pattern_binding:ident = ($instance_id:expr, $pattern:expr); )*
        }
        threads { $($threads:tt)* }
    } => {{
        (|| -> ::core::result::Result<$crate::Weave, $crate::BuildError> {
            // Duplicate bindings become duplicate struct fields, which makes
            // this authoring mistake a compile error instead of shadowing.
            #[allow(dead_code, non_camel_case_types)]
            struct __WeaveBindingNames {
                $( $knot: (), )*
                $( $pattern_binding: (), )*
            }

            let mut __builder = $crate::WeaveBuilder::new($id)?;
            $( __builder.set_numeric($numeric)?; )?
            $(
                let $knot = __builder.knot(
                    $crate::__weave_author_id!($knot $(as $alias)?),
                    $kind,
                )?;
            )*
            $(
                let $pattern_binding = __builder.include($instance_id, $pattern)?;
            )*
            $crate::__weave_threads!(__builder; $($threads)*);
            Ok(__builder.build()?)
        })()
    }};
}

/// Implementation detail for [`weave!`](crate::weave).
#[doc(hidden)]
#[macro_export]
macro_rules! __weave_author_id {
    ($binding:ident as $alias:literal) => {
        $alias
    };
    ($binding:ident) => {
        ::core::stringify!($binding)
    };
}

/// Implementation detail for [`weave!`](crate::weave).
#[doc(hidden)]
#[macro_export]
macro_rules! __weave_threads {
    ($builder:ident;) => {};

    ($builder:ident;
        $from:ident . out($from_export:expr) ->
        $to:ident . in($to_export:expr);
        $($rest:tt)*
    ) => {
        let __from = $from.output($from_export)?;
        let __to = $to.input($to_export)?;
        $builder.connect(__from, __to)?;
        $crate::__weave_threads!($builder; $($rest)*);
    };

    ($builder:ident;
        $from:ident . out($from_export:expr) ->
        $to:ident . $to_port:ident;
        $($rest:tt)*
    ) => {
        let __from = $from.output($from_export)?;
        let __to = $builder.input(&$to, ::core::stringify!($to_port))?;
        $builder.connect(__from, __to)?;
        $crate::__weave_threads!($builder; $($rest)*);
    };

    ($builder:ident;
        $from:ident . $from_port:ident ->
        $to:ident . in($to_export:expr);
        $($rest:tt)*
    ) => {
        let __from = $builder.output(&$from, ::core::stringify!($from_port))?;
        let __to = $to.input($to_export)?;
        $builder.connect(__from, __to)?;
        $crate::__weave_threads!($builder; $($rest)*);
    };

    ($builder:ident;
        $from:ident . $from_port:ident ->
        $to:ident . $to_port:ident;
        $($rest:tt)*
    ) => {
        let __from = $builder.output(&$from, ::core::stringify!($from_port))?;
        let __to = $builder.input(&$to, ::core::stringify!($to_port))?;
        $builder.connect(__from, __to)?;
        $crate::__weave_threads!($builder; $($rest)*);
    };
}
