//! Authoring macros: [`pattern!`](crate::pattern), [`weave!`](crate::weave),
//! and hidden expand helpers.
//!
//! The public surface lowers declarative definitions into the existing validated
//! authoring types while retaining compile-time binding names.

/// Builds and validates a reusable [`Pattern`](crate::Pattern).
///
/// The declaration order is `id`, optional `numeric`, `knots`, `exports`, then
/// `threads`. Exports name input and output ports that the enclosing
/// [`weave!`](crate::weave) can connect through `in(name)` and `out(name)`.
/// Knot aliases control the authored ids used by exports and threads.
///
/// The macro evaluates each expression once in source order and delegates all
/// structural and export validation to [`Pattern::try_from`](crate::Pattern).
/// It returns `Result<Pattern, BuildError>`.
///
/// ```
/// use wyrd::{pattern, KnotKind, Pattern};
///
/// fn pulse() -> Result<Pattern, wyrd::BuildError> {
///     pattern! {
///         id: "pulse";
///         knots {
///             edge as "edge.detect" = KnotKind::rising_from_zero();
///             timer = KnotKind::timer(wyrd::TimerMode::PulseHold, 2);
///         }
///         exports {
///             input start = edge.in;
///             output active = timer.active;
///         }
///         threads {
///             edge.out -> timer.start;
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! pattern {
    {
        id: $id:expr;
        numeric: $numeric:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        exports {
            $( input $input_name:ident = $input_knot:ident . $input_port:ident; )*
            $( output $output_name:ident = $output_knot:ident . $output_port:ident; )*
        }
        threads {
            $( $from:ident . $from_port:ident -> $to:ident . $to_port:ident; )*
        }
    } => {
        $crate::__pattern_expand! {
            id: $id;
            numeric: $numeric;
            knots { $( $knot $(as $alias)? = $kind; )* }
            exports {
                $( input $input_name = $input_knot . $input_port; )*
                $( output $output_name = $output_knot . $output_port; )*
            }
            threads { $( $from . $from_port -> $to . $to_port; )* }
        }
    };

    {
        id: $id:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        exports {
            $( input $input_name:ident = $input_knot:ident . $input_port:ident; )*
            $( output $output_name:ident = $output_knot:ident . $output_port:ident; )*
        }
        threads {
            $( $from:ident . $from_port:ident -> $to:ident . $to_port:ident; )*
        }
    } => {
        $crate::__pattern_expand! {
            id: $id;
            numeric: $crate::NumericPath::compiled();
            knots { $( $knot $(as $alias)? = $kind; )* }
            exports {
                $( input $input_name = $input_knot . $input_port; )*
                $( output $output_name = $output_knot . $output_port; )*
            }
            threads { $( $from . $from_port -> $to . $to_port; )* }
        }
    };
}

/// Implementation detail for [`pattern!`](crate::pattern).
#[doc(hidden)]
#[macro_export]
macro_rules! __pattern_expand {
    {
        id: $id:expr;
        numeric: $numeric:expr;
        knots {
            $( $knot:ident $(as $alias:literal)? = $kind:expr; )*
        }
        exports {
            $( input $input_name:ident = $input_knot:ident . $input_port:ident; )*
            $( output $output_name:ident = $output_knot:ident . $output_port:ident; )*
        }
        threads {
            $( $from:ident . $from_port:ident -> $to:ident . $to_port:ident; )*
        }
    } => {{
        (|| -> ::core::result::Result<$crate::Pattern, $crate::BuildError> {
            // Duplicate bindings become duplicate struct fields, which makes
            // this authoring mistake a compile error instead of shadowing.
            #[allow(dead_code, non_camel_case_types)]
            struct __PatternBindingNames {
                $( $knot: (), )*
            }

            $( let $knot = $crate::__weave_author_id!($knot $(as $alias)?); )*
            let __id: ::std::string::String = ($id).into();
            let __numeric = $numeric;
            Ok(<$crate::Pattern as ::core::convert::TryFrom<$crate::PatternDef>>::try_from(
                $crate::PatternDef {
                    id: __id.clone(),
                    inner: $crate::WeaveDef {
                        id: ::std::format!("{__id}.inner"),
                        numeric: __numeric,
                        knots: ::std::vec![
                            $( $crate::KnotDef {
                                id: $knot.into(),
                                kind: $kind,
                            }, )*
                        ],
                        threads: ::std::vec![
                            $( $crate::ThreadDef {
                                from: $crate::PortRefDef::new($from, ::core::stringify!($from_port)),
                                to: $crate::PortRefDef::new($to, ::core::stringify!($to_port)),
                            }, )*
                        ],
                    },
                    inputs: ::std::vec![
                        $( $crate::PatternExportDef::new(
                            ::core::stringify!($input_name),
                            $input_knot,
                            ::core::stringify!($input_port),
                        ), )*
                    ],
                    outputs: ::std::vec![
                        $( $crate::PatternExportDef::new(
                            ::core::stringify!($output_name),
                            $output_knot,
                            ::core::stringify!($output_port),
                        ), )*
                    ],
                },
            )?)
        })()
    }};
}

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
/// use wyrd::{weave, BuildError, KnotKind, SignalDomain, Weave};
///
/// fn graph() -> Result<Weave, BuildError> {
///     weave! {
///         id: "inverter";
///         knots {
///             source = KnotKind::signal_in(SignalDomain::Bool);
///             invert = KnotKind::not();
///             sink as "debug.inverted" = KnotKind::signal_out("debug.inverted", SignalDomain::Bool);
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
