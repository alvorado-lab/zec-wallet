// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'swap.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ExactSide {

 SwapAmount get amount;
/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ExactSideCopyWith<ExactSide> get copyWith => _$ExactSideCopyWithImpl<ExactSide>(this as ExactSide, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExactSide&&(identical(other.amount, amount) || other.amount == amount));
}


@override
int get hashCode => Object.hash(runtimeType,amount);

@override
String toString() {
  return 'ExactSide(amount: $amount)';
}


}

/// @nodoc
abstract mixin class $ExactSideCopyWith<$Res>  {
  factory $ExactSideCopyWith(ExactSide value, $Res Function(ExactSide) _then) = _$ExactSideCopyWithImpl;
@useResult
$Res call({
 SwapAmount amount
});


$SwapAmountCopyWith<$Res> get amount;

}
/// @nodoc
class _$ExactSideCopyWithImpl<$Res>
    implements $ExactSideCopyWith<$Res> {
  _$ExactSideCopyWithImpl(this._self, this._then);

  final ExactSide _self;
  final $Res Function(ExactSide) _then;

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') @override $Res call({Object? amount = null,}) {
  return _then(_self.copyWith(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as SwapAmount,
  ));
}
/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$SwapAmountCopyWith<$Res> get amount {
  
  return $SwapAmountCopyWith<$Res>(_self.amount, (value) {
    return _then(_self.copyWith(amount: value));
  });
}
}


/// Adds pattern-matching-related methods to [ExactSide].
extension ExactSidePatterns on ExactSide {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ExactSide_In value)?  in_,TResult Function( ExactSide_Out value)?  out,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ExactSide_In() when in_ != null:
return in_(_that);case ExactSide_Out() when out != null:
return out(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ExactSide_In value)  in_,required TResult Function( ExactSide_Out value)  out,}){
final _that = this;
switch (_that) {
case ExactSide_In():
return in_(_that);case ExactSide_Out():
return out(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ExactSide_In value)?  in_,TResult? Function( ExactSide_Out value)?  out,}){
final _that = this;
switch (_that) {
case ExactSide_In() when in_ != null:
return in_(_that);case ExactSide_Out() when out != null:
return out(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( SwapAmount amount)?  in_,TResult Function( SwapAmount amount)?  out,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ExactSide_In() when in_ != null:
return in_(_that.amount);case ExactSide_Out() when out != null:
return out(_that.amount);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( SwapAmount amount)  in_,required TResult Function( SwapAmount amount)  out,}) {final _that = this;
switch (_that) {
case ExactSide_In():
return in_(_that.amount);case ExactSide_Out():
return out(_that.amount);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( SwapAmount amount)?  in_,TResult? Function( SwapAmount amount)?  out,}) {final _that = this;
switch (_that) {
case ExactSide_In() when in_ != null:
return in_(_that.amount);case ExactSide_Out() when out != null:
return out(_that.amount);case _:
  return null;

}
}

}

/// @nodoc


class ExactSide_In extends ExactSide {
  const ExactSide_In({required this.amount}): super._();
  

@override final  SwapAmount amount;

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ExactSide_InCopyWith<ExactSide_In> get copyWith => _$ExactSide_InCopyWithImpl<ExactSide_In>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExactSide_In&&(identical(other.amount, amount) || other.amount == amount));
}


@override
int get hashCode => Object.hash(runtimeType,amount);

@override
String toString() {
  return 'ExactSide.in_(amount: $amount)';
}


}

/// @nodoc
abstract mixin class $ExactSide_InCopyWith<$Res> implements $ExactSideCopyWith<$Res> {
  factory $ExactSide_InCopyWith(ExactSide_In value, $Res Function(ExactSide_In) _then) = _$ExactSide_InCopyWithImpl;
@override @useResult
$Res call({
 SwapAmount amount
});


@override $SwapAmountCopyWith<$Res> get amount;

}
/// @nodoc
class _$ExactSide_InCopyWithImpl<$Res>
    implements $ExactSide_InCopyWith<$Res> {
  _$ExactSide_InCopyWithImpl(this._self, this._then);

  final ExactSide_In _self;
  final $Res Function(ExactSide_In) _then;

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? amount = null,}) {
  return _then(ExactSide_In(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as SwapAmount,
  ));
}

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$SwapAmountCopyWith<$Res> get amount {
  
  return $SwapAmountCopyWith<$Res>(_self.amount, (value) {
    return _then(_self.copyWith(amount: value));
  });
}
}

/// @nodoc


class ExactSide_Out extends ExactSide {
  const ExactSide_Out({required this.amount}): super._();
  

@override final  SwapAmount amount;

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override @JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ExactSide_OutCopyWith<ExactSide_Out> get copyWith => _$ExactSide_OutCopyWithImpl<ExactSide_Out>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ExactSide_Out&&(identical(other.amount, amount) || other.amount == amount));
}


@override
int get hashCode => Object.hash(runtimeType,amount);

@override
String toString() {
  return 'ExactSide.out(amount: $amount)';
}


}

/// @nodoc
abstract mixin class $ExactSide_OutCopyWith<$Res> implements $ExactSideCopyWith<$Res> {
  factory $ExactSide_OutCopyWith(ExactSide_Out value, $Res Function(ExactSide_Out) _then) = _$ExactSide_OutCopyWithImpl;
@override @useResult
$Res call({
 SwapAmount amount
});


@override $SwapAmountCopyWith<$Res> get amount;

}
/// @nodoc
class _$ExactSide_OutCopyWithImpl<$Res>
    implements $ExactSide_OutCopyWith<$Res> {
  _$ExactSide_OutCopyWithImpl(this._self, this._then);

  final ExactSide_Out _self;
  final $Res Function(ExactSide_Out) _then;

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override @pragma('vm:prefer-inline') $Res call({Object? amount = null,}) {
  return _then(ExactSide_Out(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as SwapAmount,
  ));
}

/// Create a copy of ExactSide
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$SwapAmountCopyWith<$Res> get amount {
  
  return $SwapAmountCopyWith<$Res>(_self.amount, (value) {
    return _then(_self.copyWith(amount: value));
  });
}
}

/// @nodoc
mixin _$SwapAmount {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapAmount);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapAmount()';
}


}

/// @nodoc
class $SwapAmountCopyWith<$Res>  {
$SwapAmountCopyWith(SwapAmount _, $Res Function(SwapAmount) __);
}


/// Adds pattern-matching-related methods to [SwapAmount].
extension SwapAmountPatterns on SwapAmount {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SwapAmount_Zec value)?  zec,TResult Function( SwapAmount_Foreign value)?  foreign,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SwapAmount_Zec() when zec != null:
return zec(_that);case SwapAmount_Foreign() when foreign != null:
return foreign(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SwapAmount_Zec value)  zec,required TResult Function( SwapAmount_Foreign value)  foreign,}){
final _that = this;
switch (_that) {
case SwapAmount_Zec():
return zec(_that);case SwapAmount_Foreign():
return foreign(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SwapAmount_Zec value)?  zec,TResult? Function( SwapAmount_Foreign value)?  foreign,}){
final _that = this;
switch (_that) {
case SwapAmount_Zec() when zec != null:
return zec(_that);case SwapAmount_Foreign() when foreign != null:
return foreign(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PlatformInt64 zat)?  zec,TResult Function( String amount)?  foreign,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SwapAmount_Zec() when zec != null:
return zec(_that.zat);case SwapAmount_Foreign() when foreign != null:
return foreign(_that.amount);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PlatformInt64 zat)  zec,required TResult Function( String amount)  foreign,}) {final _that = this;
switch (_that) {
case SwapAmount_Zec():
return zec(_that.zat);case SwapAmount_Foreign():
return foreign(_that.amount);}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PlatformInt64 zat)?  zec,TResult? Function( String amount)?  foreign,}) {final _that = this;
switch (_that) {
case SwapAmount_Zec() when zec != null:
return zec(_that.zat);case SwapAmount_Foreign() when foreign != null:
return foreign(_that.amount);case _:
  return null;

}
}

}

/// @nodoc


class SwapAmount_Zec extends SwapAmount {
  const SwapAmount_Zec({required this.zat}): super._();
  

 final  PlatformInt64 zat;

/// Create a copy of SwapAmount
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapAmount_ZecCopyWith<SwapAmount_Zec> get copyWith => _$SwapAmount_ZecCopyWithImpl<SwapAmount_Zec>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapAmount_Zec&&(identical(other.zat, zat) || other.zat == zat));
}


@override
int get hashCode => Object.hash(runtimeType,zat);

@override
String toString() {
  return 'SwapAmount.zec(zat: $zat)';
}


}

/// @nodoc
abstract mixin class $SwapAmount_ZecCopyWith<$Res> implements $SwapAmountCopyWith<$Res> {
  factory $SwapAmount_ZecCopyWith(SwapAmount_Zec value, $Res Function(SwapAmount_Zec) _then) = _$SwapAmount_ZecCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 zat
});




}
/// @nodoc
class _$SwapAmount_ZecCopyWithImpl<$Res>
    implements $SwapAmount_ZecCopyWith<$Res> {
  _$SwapAmount_ZecCopyWithImpl(this._self, this._then);

  final SwapAmount_Zec _self;
  final $Res Function(SwapAmount_Zec) _then;

/// Create a copy of SwapAmount
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? zat = null,}) {
  return _then(SwapAmount_Zec(
zat: null == zat ? _self.zat : zat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class SwapAmount_Foreign extends SwapAmount {
  const SwapAmount_Foreign({required this.amount}): super._();
  

 final  String amount;

/// Create a copy of SwapAmount
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapAmount_ForeignCopyWith<SwapAmount_Foreign> get copyWith => _$SwapAmount_ForeignCopyWithImpl<SwapAmount_Foreign>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapAmount_Foreign&&(identical(other.amount, amount) || other.amount == amount));
}


@override
int get hashCode => Object.hash(runtimeType,amount);

@override
String toString() {
  return 'SwapAmount.foreign(amount: $amount)';
}


}

/// @nodoc
abstract mixin class $SwapAmount_ForeignCopyWith<$Res> implements $SwapAmountCopyWith<$Res> {
  factory $SwapAmount_ForeignCopyWith(SwapAmount_Foreign value, $Res Function(SwapAmount_Foreign) _then) = _$SwapAmount_ForeignCopyWithImpl;
@useResult
$Res call({
 String amount
});




}
/// @nodoc
class _$SwapAmount_ForeignCopyWithImpl<$Res>
    implements $SwapAmount_ForeignCopyWith<$Res> {
  _$SwapAmount_ForeignCopyWithImpl(this._self, this._then);

  final SwapAmount_Foreign _self;
  final $Res Function(SwapAmount_Foreign) _then;

/// Create a copy of SwapAmount
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? amount = null,}) {
  return _then(SwapAmount_Foreign(
amount: null == amount ? _self.amount : amount // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc
mixin _$SwapDirection {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapDirection);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapDirection()';
}


}

/// @nodoc
class $SwapDirectionCopyWith<$Res>  {
$SwapDirectionCopyWith(SwapDirection _, $Res Function(SwapDirection) __);
}


/// Adds pattern-matching-related methods to [SwapDirection].
extension SwapDirectionPatterns on SwapDirection {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SwapDirection_IntoZec value)?  intoZec,TResult Function( SwapDirection_OutOfZec value)?  outOfZec,TResult Function( SwapDirection_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SwapDirection_IntoZec() when intoZec != null:
return intoZec(_that);case SwapDirection_OutOfZec() when outOfZec != null:
return outOfZec(_that);case SwapDirection_Unknown() when unknown != null:
return unknown(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SwapDirection_IntoZec value)  intoZec,required TResult Function( SwapDirection_OutOfZec value)  outOfZec,required TResult Function( SwapDirection_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SwapDirection_IntoZec():
return intoZec(_that);case SwapDirection_OutOfZec():
return outOfZec(_that);case SwapDirection_Unknown():
return unknown(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SwapDirection_IntoZec value)?  intoZec,TResult? Function( SwapDirection_OutOfZec value)?  outOfZec,TResult? Function( SwapDirection_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SwapDirection_IntoZec() when intoZec != null:
return intoZec(_that);case SwapDirection_OutOfZec() when outOfZec != null:
return outOfZec(_that);case SwapDirection_Unknown() when unknown != null:
return unknown(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( AssetId from)?  intoZec,TResult Function( AssetId to)?  outOfZec,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SwapDirection_IntoZec() when intoZec != null:
return intoZec(_that.from);case SwapDirection_OutOfZec() when outOfZec != null:
return outOfZec(_that.to);case SwapDirection_Unknown() when unknown != null:
return unknown();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( AssetId from)  intoZec,required TResult Function( AssetId to)  outOfZec,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SwapDirection_IntoZec():
return intoZec(_that.from);case SwapDirection_OutOfZec():
return outOfZec(_that.to);case SwapDirection_Unknown():
return unknown();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( AssetId from)?  intoZec,TResult? Function( AssetId to)?  outOfZec,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SwapDirection_IntoZec() when intoZec != null:
return intoZec(_that.from);case SwapDirection_OutOfZec() when outOfZec != null:
return outOfZec(_that.to);case SwapDirection_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SwapDirection_IntoZec extends SwapDirection {
  const SwapDirection_IntoZec({required this.from}): super._();
  

 final  AssetId from;

/// Create a copy of SwapDirection
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapDirection_IntoZecCopyWith<SwapDirection_IntoZec> get copyWith => _$SwapDirection_IntoZecCopyWithImpl<SwapDirection_IntoZec>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapDirection_IntoZec&&(identical(other.from, from) || other.from == from));
}


@override
int get hashCode => Object.hash(runtimeType,from);

@override
String toString() {
  return 'SwapDirection.intoZec(from: $from)';
}


}

/// @nodoc
abstract mixin class $SwapDirection_IntoZecCopyWith<$Res> implements $SwapDirectionCopyWith<$Res> {
  factory $SwapDirection_IntoZecCopyWith(SwapDirection_IntoZec value, $Res Function(SwapDirection_IntoZec) _then) = _$SwapDirection_IntoZecCopyWithImpl;
@useResult
$Res call({
 AssetId from
});




}
/// @nodoc
class _$SwapDirection_IntoZecCopyWithImpl<$Res>
    implements $SwapDirection_IntoZecCopyWith<$Res> {
  _$SwapDirection_IntoZecCopyWithImpl(this._self, this._then);

  final SwapDirection_IntoZec _self;
  final $Res Function(SwapDirection_IntoZec) _then;

/// Create a copy of SwapDirection
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,}) {
  return _then(SwapDirection_IntoZec(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as AssetId,
  ));
}


}

/// @nodoc


class SwapDirection_OutOfZec extends SwapDirection {
  const SwapDirection_OutOfZec({required this.to}): super._();
  

 final  AssetId to;

/// Create a copy of SwapDirection
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapDirection_OutOfZecCopyWith<SwapDirection_OutOfZec> get copyWith => _$SwapDirection_OutOfZecCopyWithImpl<SwapDirection_OutOfZec>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapDirection_OutOfZec&&(identical(other.to, to) || other.to == to));
}


@override
int get hashCode => Object.hash(runtimeType,to);

@override
String toString() {
  return 'SwapDirection.outOfZec(to: $to)';
}


}

/// @nodoc
abstract mixin class $SwapDirection_OutOfZecCopyWith<$Res> implements $SwapDirectionCopyWith<$Res> {
  factory $SwapDirection_OutOfZecCopyWith(SwapDirection_OutOfZec value, $Res Function(SwapDirection_OutOfZec) _then) = _$SwapDirection_OutOfZecCopyWithImpl;
@useResult
$Res call({
 AssetId to
});




}
/// @nodoc
class _$SwapDirection_OutOfZecCopyWithImpl<$Res>
    implements $SwapDirection_OutOfZecCopyWith<$Res> {
  _$SwapDirection_OutOfZecCopyWithImpl(this._self, this._then);

  final SwapDirection_OutOfZec _self;
  final $Res Function(SwapDirection_OutOfZec) _then;

/// Create a copy of SwapDirection
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? to = null,}) {
  return _then(SwapDirection_OutOfZec(
to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as AssetId,
  ));
}


}

/// @nodoc


class SwapDirection_Unknown extends SwapDirection {
  const SwapDirection_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapDirection_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapDirection.unknown()';
}


}




/// @nodoc
mixin _$SwapStatus {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapStatus()';
}


}

/// @nodoc
class $SwapStatusCopyWith<$Res>  {
$SwapStatusCopyWith(SwapStatus _, $Res Function(SwapStatus) __);
}


/// Adds pattern-matching-related methods to [SwapStatus].
extension SwapStatusPatterns on SwapStatus {
/// A variant of `map` that fallback to returning `orElse`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SwapStatus_PendingDeposit value)?  pendingDeposit,TResult Function( SwapStatus_UnderDeposited value)?  underDeposited,TResult Function( SwapStatus_DepositDetected value)?  depositDetected,TResult Function( SwapStatus_Processing value)?  processing,TResult Function( SwapStatus_Success value)?  success,TResult Function( SwapStatus_Refunded value)?  refunded,TResult Function( SwapStatus_Failed value)?  failed,TResult Function( SwapStatus_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit() when pendingDeposit != null:
return pendingDeposit(_that);case SwapStatus_UnderDeposited() when underDeposited != null:
return underDeposited(_that);case SwapStatus_DepositDetected() when depositDetected != null:
return depositDetected(_that);case SwapStatus_Processing() when processing != null:
return processing(_that);case SwapStatus_Success() when success != null:
return success(_that);case SwapStatus_Refunded() when refunded != null:
return refunded(_that);case SwapStatus_Failed() when failed != null:
return failed(_that);case SwapStatus_Unknown() when unknown != null:
return unknown(_that);case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// Callbacks receives the raw object, upcasted.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case final Subclass2 value:
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SwapStatus_PendingDeposit value)  pendingDeposit,required TResult Function( SwapStatus_UnderDeposited value)  underDeposited,required TResult Function( SwapStatus_DepositDetected value)  depositDetected,required TResult Function( SwapStatus_Processing value)  processing,required TResult Function( SwapStatus_Success value)  success,required TResult Function( SwapStatus_Refunded value)  refunded,required TResult Function( SwapStatus_Failed value)  failed,required TResult Function( SwapStatus_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit():
return pendingDeposit(_that);case SwapStatus_UnderDeposited():
return underDeposited(_that);case SwapStatus_DepositDetected():
return depositDetected(_that);case SwapStatus_Processing():
return processing(_that);case SwapStatus_Success():
return success(_that);case SwapStatus_Refunded():
return refunded(_that);case SwapStatus_Failed():
return failed(_that);case SwapStatus_Unknown():
return unknown(_that);}
}
/// A variant of `map` that fallback to returning `null`.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case final Subclass value:
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SwapStatus_PendingDeposit value)?  pendingDeposit,TResult? Function( SwapStatus_UnderDeposited value)?  underDeposited,TResult? Function( SwapStatus_DepositDetected value)?  depositDetected,TResult? Function( SwapStatus_Processing value)?  processing,TResult? Function( SwapStatus_Success value)?  success,TResult? Function( SwapStatus_Refunded value)?  refunded,TResult? Function( SwapStatus_Failed value)?  failed,TResult? Function( SwapStatus_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit() when pendingDeposit != null:
return pendingDeposit(_that);case SwapStatus_UnderDeposited() when underDeposited != null:
return underDeposited(_that);case SwapStatus_DepositDetected() when depositDetected != null:
return depositDetected(_that);case SwapStatus_Processing() when processing != null:
return processing(_that);case SwapStatus_Success() when success != null:
return success(_that);case SwapStatus_Refunded() when refunded != null:
return refunded(_that);case SwapStatus_Failed() when failed != null:
return failed(_that);case SwapStatus_Unknown() when unknown != null:
return unknown(_that);case _:
  return null;

}
}
/// A variant of `when` that fallback to an `orElse` callback.
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return orElse();
/// }
/// ```

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( PlatformInt64 expiresAt)?  pendingDeposit,TResult Function( String received,  String missing,  PlatformInt64 deadline)?  underDeposited,TResult Function()?  depositDetected,TResult Function()?  processing,TResult Function( String? outTxid,  int? realizedSlippageBps)?  success,TResult Function( String? refundTxid)?  refunded,TResult Function( SwapFailureCode code)?  failed,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit() when pendingDeposit != null:
return pendingDeposit(_that.expiresAt);case SwapStatus_UnderDeposited() when underDeposited != null:
return underDeposited(_that.received,_that.missing,_that.deadline);case SwapStatus_DepositDetected() when depositDetected != null:
return depositDetected();case SwapStatus_Processing() when processing != null:
return processing();case SwapStatus_Success() when success != null:
return success(_that.outTxid,_that.realizedSlippageBps);case SwapStatus_Refunded() when refunded != null:
return refunded(_that.refundTxid);case SwapStatus_Failed() when failed != null:
return failed(_that.code);case SwapStatus_Unknown() when unknown != null:
return unknown();case _:
  return orElse();

}
}
/// A `switch`-like method, using callbacks.
///
/// As opposed to `map`, this offers destructuring.
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case Subclass2(:final field2):
///     return ...;
/// }
/// ```

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( PlatformInt64 expiresAt)  pendingDeposit,required TResult Function( String received,  String missing,  PlatformInt64 deadline)  underDeposited,required TResult Function()  depositDetected,required TResult Function()  processing,required TResult Function( String? outTxid,  int? realizedSlippageBps)  success,required TResult Function( String? refundTxid)  refunded,required TResult Function( SwapFailureCode code)  failed,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit():
return pendingDeposit(_that.expiresAt);case SwapStatus_UnderDeposited():
return underDeposited(_that.received,_that.missing,_that.deadline);case SwapStatus_DepositDetected():
return depositDetected();case SwapStatus_Processing():
return processing();case SwapStatus_Success():
return success(_that.outTxid,_that.realizedSlippageBps);case SwapStatus_Refunded():
return refunded(_that.refundTxid);case SwapStatus_Failed():
return failed(_that.code);case SwapStatus_Unknown():
return unknown();}
}
/// A variant of `when` that fallback to returning `null`
///
/// It is equivalent to doing:
/// ```dart
/// switch (sealedClass) {
///   case Subclass(:final field):
///     return ...;
///   case _:
///     return null;
/// }
/// ```

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( PlatformInt64 expiresAt)?  pendingDeposit,TResult? Function( String received,  String missing,  PlatformInt64 deadline)?  underDeposited,TResult? Function()?  depositDetected,TResult? Function()?  processing,TResult? Function( String? outTxid,  int? realizedSlippageBps)?  success,TResult? Function( String? refundTxid)?  refunded,TResult? Function( SwapFailureCode code)?  failed,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SwapStatus_PendingDeposit() when pendingDeposit != null:
return pendingDeposit(_that.expiresAt);case SwapStatus_UnderDeposited() when underDeposited != null:
return underDeposited(_that.received,_that.missing,_that.deadline);case SwapStatus_DepositDetected() when depositDetected != null:
return depositDetected();case SwapStatus_Processing() when processing != null:
return processing();case SwapStatus_Success() when success != null:
return success(_that.outTxid,_that.realizedSlippageBps);case SwapStatus_Refunded() when refunded != null:
return refunded(_that.refundTxid);case SwapStatus_Failed() when failed != null:
return failed(_that.code);case SwapStatus_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SwapStatus_PendingDeposit extends SwapStatus {
  const SwapStatus_PendingDeposit({required this.expiresAt}): super._();
  

 final  PlatformInt64 expiresAt;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapStatus_PendingDepositCopyWith<SwapStatus_PendingDeposit> get copyWith => _$SwapStatus_PendingDepositCopyWithImpl<SwapStatus_PendingDeposit>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_PendingDeposit&&(identical(other.expiresAt, expiresAt) || other.expiresAt == expiresAt));
}


@override
int get hashCode => Object.hash(runtimeType,expiresAt);

@override
String toString() {
  return 'SwapStatus.pendingDeposit(expiresAt: $expiresAt)';
}


}

/// @nodoc
abstract mixin class $SwapStatus_PendingDepositCopyWith<$Res> implements $SwapStatusCopyWith<$Res> {
  factory $SwapStatus_PendingDepositCopyWith(SwapStatus_PendingDeposit value, $Res Function(SwapStatus_PendingDeposit) _then) = _$SwapStatus_PendingDepositCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 expiresAt
});




}
/// @nodoc
class _$SwapStatus_PendingDepositCopyWithImpl<$Res>
    implements $SwapStatus_PendingDepositCopyWith<$Res> {
  _$SwapStatus_PendingDepositCopyWithImpl(this._self, this._then);

  final SwapStatus_PendingDeposit _self;
  final $Res Function(SwapStatus_PendingDeposit) _then;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? expiresAt = null,}) {
  return _then(SwapStatus_PendingDeposit(
expiresAt: null == expiresAt ? _self.expiresAt : expiresAt // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class SwapStatus_UnderDeposited extends SwapStatus {
  const SwapStatus_UnderDeposited({required this.received, required this.missing, required this.deadline}): super._();
  

 final  String received;
 final  String missing;
 final  PlatformInt64 deadline;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapStatus_UnderDepositedCopyWith<SwapStatus_UnderDeposited> get copyWith => _$SwapStatus_UnderDepositedCopyWithImpl<SwapStatus_UnderDeposited>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_UnderDeposited&&(identical(other.received, received) || other.received == received)&&(identical(other.missing, missing) || other.missing == missing)&&(identical(other.deadline, deadline) || other.deadline == deadline));
}


@override
int get hashCode => Object.hash(runtimeType,received,missing,deadline);

@override
String toString() {
  return 'SwapStatus.underDeposited(received: $received, missing: $missing, deadline: $deadline)';
}


}

/// @nodoc
abstract mixin class $SwapStatus_UnderDepositedCopyWith<$Res> implements $SwapStatusCopyWith<$Res> {
  factory $SwapStatus_UnderDepositedCopyWith(SwapStatus_UnderDeposited value, $Res Function(SwapStatus_UnderDeposited) _then) = _$SwapStatus_UnderDepositedCopyWithImpl;
@useResult
$Res call({
 String received, String missing, PlatformInt64 deadline
});




}
/// @nodoc
class _$SwapStatus_UnderDepositedCopyWithImpl<$Res>
    implements $SwapStatus_UnderDepositedCopyWith<$Res> {
  _$SwapStatus_UnderDepositedCopyWithImpl(this._self, this._then);

  final SwapStatus_UnderDeposited _self;
  final $Res Function(SwapStatus_UnderDeposited) _then;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? received = null,Object? missing = null,Object? deadline = null,}) {
  return _then(SwapStatus_UnderDeposited(
received: null == received ? _self.received : received // ignore: cast_nullable_to_non_nullable
as String,missing: null == missing ? _self.missing : missing // ignore: cast_nullable_to_non_nullable
as String,deadline: null == deadline ? _self.deadline : deadline // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class SwapStatus_DepositDetected extends SwapStatus {
  const SwapStatus_DepositDetected(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_DepositDetected);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapStatus.depositDetected()';
}


}




/// @nodoc


class SwapStatus_Processing extends SwapStatus {
  const SwapStatus_Processing(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_Processing);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapStatus.processing()';
}


}




/// @nodoc


class SwapStatus_Success extends SwapStatus {
  const SwapStatus_Success({this.outTxid, this.realizedSlippageBps}): super._();
  

 final  String? outTxid;
 final  int? realizedSlippageBps;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapStatus_SuccessCopyWith<SwapStatus_Success> get copyWith => _$SwapStatus_SuccessCopyWithImpl<SwapStatus_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_Success&&(identical(other.outTxid, outTxid) || other.outTxid == outTxid)&&(identical(other.realizedSlippageBps, realizedSlippageBps) || other.realizedSlippageBps == realizedSlippageBps));
}


@override
int get hashCode => Object.hash(runtimeType,outTxid,realizedSlippageBps);

@override
String toString() {
  return 'SwapStatus.success(outTxid: $outTxid, realizedSlippageBps: $realizedSlippageBps)';
}


}

/// @nodoc
abstract mixin class $SwapStatus_SuccessCopyWith<$Res> implements $SwapStatusCopyWith<$Res> {
  factory $SwapStatus_SuccessCopyWith(SwapStatus_Success value, $Res Function(SwapStatus_Success) _then) = _$SwapStatus_SuccessCopyWithImpl;
@useResult
$Res call({
 String? outTxid, int? realizedSlippageBps
});




}
/// @nodoc
class _$SwapStatus_SuccessCopyWithImpl<$Res>
    implements $SwapStatus_SuccessCopyWith<$Res> {
  _$SwapStatus_SuccessCopyWithImpl(this._self, this._then);

  final SwapStatus_Success _self;
  final $Res Function(SwapStatus_Success) _then;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? outTxid = freezed,Object? realizedSlippageBps = freezed,}) {
  return _then(SwapStatus_Success(
outTxid: freezed == outTxid ? _self.outTxid : outTxid // ignore: cast_nullable_to_non_nullable
as String?,realizedSlippageBps: freezed == realizedSlippageBps ? _self.realizedSlippageBps : realizedSlippageBps // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class SwapStatus_Refunded extends SwapStatus {
  const SwapStatus_Refunded({this.refundTxid}): super._();
  

 final  String? refundTxid;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapStatus_RefundedCopyWith<SwapStatus_Refunded> get copyWith => _$SwapStatus_RefundedCopyWithImpl<SwapStatus_Refunded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_Refunded&&(identical(other.refundTxid, refundTxid) || other.refundTxid == refundTxid));
}


@override
int get hashCode => Object.hash(runtimeType,refundTxid);

@override
String toString() {
  return 'SwapStatus.refunded(refundTxid: $refundTxid)';
}


}

/// @nodoc
abstract mixin class $SwapStatus_RefundedCopyWith<$Res> implements $SwapStatusCopyWith<$Res> {
  factory $SwapStatus_RefundedCopyWith(SwapStatus_Refunded value, $Res Function(SwapStatus_Refunded) _then) = _$SwapStatus_RefundedCopyWithImpl;
@useResult
$Res call({
 String? refundTxid
});




}
/// @nodoc
class _$SwapStatus_RefundedCopyWithImpl<$Res>
    implements $SwapStatus_RefundedCopyWith<$Res> {
  _$SwapStatus_RefundedCopyWithImpl(this._self, this._then);

  final SwapStatus_Refunded _self;
  final $Res Function(SwapStatus_Refunded) _then;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? refundTxid = freezed,}) {
  return _then(SwapStatus_Refunded(
refundTxid: freezed == refundTxid ? _self.refundTxid : refundTxid // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class SwapStatus_Failed extends SwapStatus {
  const SwapStatus_Failed({required this.code}): super._();
  

 final  SwapFailureCode code;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapStatus_FailedCopyWith<SwapStatus_Failed> get copyWith => _$SwapStatus_FailedCopyWithImpl<SwapStatus_Failed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_Failed&&(identical(other.code, code) || other.code == code));
}


@override
int get hashCode => Object.hash(runtimeType,code);

@override
String toString() {
  return 'SwapStatus.failed(code: $code)';
}


}

/// @nodoc
abstract mixin class $SwapStatus_FailedCopyWith<$Res> implements $SwapStatusCopyWith<$Res> {
  factory $SwapStatus_FailedCopyWith(SwapStatus_Failed value, $Res Function(SwapStatus_Failed) _then) = _$SwapStatus_FailedCopyWithImpl;
@useResult
$Res call({
 SwapFailureCode code
});




}
/// @nodoc
class _$SwapStatus_FailedCopyWithImpl<$Res>
    implements $SwapStatus_FailedCopyWith<$Res> {
  _$SwapStatus_FailedCopyWithImpl(this._self, this._then);

  final SwapStatus_Failed _self;
  final $Res Function(SwapStatus_Failed) _then;

/// Create a copy of SwapStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? code = null,}) {
  return _then(SwapStatus_Failed(
code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as SwapFailureCode,
  ));
}


}

/// @nodoc


class SwapStatus_Unknown extends SwapStatus {
  const SwapStatus_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapStatus_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapStatus.unknown()';
}


}




// dart format on
