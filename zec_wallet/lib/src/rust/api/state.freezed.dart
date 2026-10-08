// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'state.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$PoolService {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PoolService()';
}


}

/// @nodoc
class $PoolServiceCopyWith<$Res>  {
$PoolServiceCopyWith(PoolService _, $Res Function(PoolService) __);
}


/// Adds pattern-matching-related methods to [PoolService].
extension PoolServicePatterns on PoolService {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( PoolService_Served value)?  served,TResult Function( PoolService_Withheld value)?  withheld,TResult Function( PoolService_Unsupported value)?  unsupported,TResult Function( PoolService_HeightViolation value)?  heightViolation,TResult Function( PoolService_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case PoolService_Served() when served != null:
return served(_that);case PoolService_Withheld() when withheld != null:
return withheld(_that);case PoolService_Unsupported() when unsupported != null:
return unsupported(_that);case PoolService_HeightViolation() when heightViolation != null:
return heightViolation(_that);case PoolService_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( PoolService_Served value)  served,required TResult Function( PoolService_Withheld value)  withheld,required TResult Function( PoolService_Unsupported value)  unsupported,required TResult Function( PoolService_HeightViolation value)  heightViolation,required TResult Function( PoolService_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case PoolService_Served():
return served(_that);case PoolService_Withheld():
return withheld(_that);case PoolService_Unsupported():
return unsupported(_that);case PoolService_HeightViolation():
return heightViolation(_that);case PoolService_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( PoolService_Served value)?  served,TResult? Function( PoolService_Withheld value)?  withheld,TResult? Function( PoolService_Unsupported value)?  unsupported,TResult? Function( PoolService_HeightViolation value)?  heightViolation,TResult? Function( PoolService_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case PoolService_Served() when served != null:
return served(_that);case PoolService_Withheld() when withheld != null:
return withheld(_that);case PoolService_Unsupported() when unsupported != null:
return unsupported(_that);case PoolService_HeightViolation() when heightViolation != null:
return heightViolation(_that);case PoolService_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int roots)?  served,TResult Function( int proven)?  withheld,TResult Function()?  unsupported,TResult Function()?  heightViolation,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case PoolService_Served() when served != null:
return served(_that.roots);case PoolService_Withheld() when withheld != null:
return withheld(_that.proven);case PoolService_Unsupported() when unsupported != null:
return unsupported();case PoolService_HeightViolation() when heightViolation != null:
return heightViolation();case PoolService_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int roots)  served,required TResult Function( int proven)  withheld,required TResult Function()  unsupported,required TResult Function()  heightViolation,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case PoolService_Served():
return served(_that.roots);case PoolService_Withheld():
return withheld(_that.proven);case PoolService_Unsupported():
return unsupported();case PoolService_HeightViolation():
return heightViolation();case PoolService_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int roots)?  served,TResult? Function( int proven)?  withheld,TResult? Function()?  unsupported,TResult? Function()?  heightViolation,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case PoolService_Served() when served != null:
return served(_that.roots);case PoolService_Withheld() when withheld != null:
return withheld(_that.proven);case PoolService_Unsupported() when unsupported != null:
return unsupported();case PoolService_HeightViolation() when heightViolation != null:
return heightViolation();case PoolService_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class PoolService_Served extends PoolService {
  const PoolService_Served({required this.roots}): super._();
  

 final  int roots;

/// Create a copy of PoolService
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PoolService_ServedCopyWith<PoolService_Served> get copyWith => _$PoolService_ServedCopyWithImpl<PoolService_Served>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService_Served&&(identical(other.roots, roots) || other.roots == roots));
}


@override
int get hashCode => Object.hash(runtimeType,roots);

@override
String toString() {
  return 'PoolService.served(roots: $roots)';
}


}

/// @nodoc
abstract mixin class $PoolService_ServedCopyWith<$Res> implements $PoolServiceCopyWith<$Res> {
  factory $PoolService_ServedCopyWith(PoolService_Served value, $Res Function(PoolService_Served) _then) = _$PoolService_ServedCopyWithImpl;
@useResult
$Res call({
 int roots
});




}
/// @nodoc
class _$PoolService_ServedCopyWithImpl<$Res>
    implements $PoolService_ServedCopyWith<$Res> {
  _$PoolService_ServedCopyWithImpl(this._self, this._then);

  final PoolService_Served _self;
  final $Res Function(PoolService_Served) _then;

/// Create a copy of PoolService
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? roots = null,}) {
  return _then(PoolService_Served(
roots: null == roots ? _self.roots : roots // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class PoolService_Withheld extends PoolService {
  const PoolService_Withheld({required this.proven}): super._();
  

 final  int proven;

/// Create a copy of PoolService
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$PoolService_WithheldCopyWith<PoolService_Withheld> get copyWith => _$PoolService_WithheldCopyWithImpl<PoolService_Withheld>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService_Withheld&&(identical(other.proven, proven) || other.proven == proven));
}


@override
int get hashCode => Object.hash(runtimeType,proven);

@override
String toString() {
  return 'PoolService.withheld(proven: $proven)';
}


}

/// @nodoc
abstract mixin class $PoolService_WithheldCopyWith<$Res> implements $PoolServiceCopyWith<$Res> {
  factory $PoolService_WithheldCopyWith(PoolService_Withheld value, $Res Function(PoolService_Withheld) _then) = _$PoolService_WithheldCopyWithImpl;
@useResult
$Res call({
 int proven
});




}
/// @nodoc
class _$PoolService_WithheldCopyWithImpl<$Res>
    implements $PoolService_WithheldCopyWith<$Res> {
  _$PoolService_WithheldCopyWithImpl(this._self, this._then);

  final PoolService_Withheld _self;
  final $Res Function(PoolService_Withheld) _then;

/// Create a copy of PoolService
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? proven = null,}) {
  return _then(PoolService_Withheld(
proven: null == proven ? _self.proven : proven // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class PoolService_Unsupported extends PoolService {
  const PoolService_Unsupported(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService_Unsupported);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PoolService.unsupported()';
}


}




/// @nodoc


class PoolService_HeightViolation extends PoolService {
  const PoolService_HeightViolation(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService_HeightViolation);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PoolService.heightViolation()';
}


}




/// @nodoc


class PoolService_Unknown extends PoolService {
  const PoolService_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is PoolService_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'PoolService.unknown()';
}


}




/// @nodoc
mixin _$ReclaimOutcome {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ReclaimOutcome);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ReclaimOutcome()';
}


}

/// @nodoc
class $ReclaimOutcomeCopyWith<$Res>  {
$ReclaimOutcomeCopyWith(ReclaimOutcome _, $Res Function(ReclaimOutcome) __);
}


/// Adds pattern-matching-related methods to [ReclaimOutcome].
extension ReclaimOutcomePatterns on ReclaimOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ReclaimOutcome_NothingToReclaim value)?  nothingToReclaim,TResult Function( ReclaimOutcome_Minted value)?  minted,TResult Function( ReclaimOutcome_NotBroadcast value)?  notBroadcast,TResult Function( ReclaimOutcome_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim() when nothingToReclaim != null:
return nothingToReclaim(_that);case ReclaimOutcome_Minted() when minted != null:
return minted(_that);case ReclaimOutcome_NotBroadcast() when notBroadcast != null:
return notBroadcast(_that);case ReclaimOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ReclaimOutcome_NothingToReclaim value)  nothingToReclaim,required TResult Function( ReclaimOutcome_Minted value)  minted,required TResult Function( ReclaimOutcome_NotBroadcast value)  notBroadcast,required TResult Function( ReclaimOutcome_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim():
return nothingToReclaim(_that);case ReclaimOutcome_Minted():
return minted(_that);case ReclaimOutcome_NotBroadcast():
return notBroadcast(_that);case ReclaimOutcome_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ReclaimOutcome_NothingToReclaim value)?  nothingToReclaim,TResult? Function( ReclaimOutcome_Minted value)?  minted,TResult? Function( ReclaimOutcome_NotBroadcast value)?  notBroadcast,TResult? Function( ReclaimOutcome_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim() when nothingToReclaim != null:
return nothingToReclaim(_that);case ReclaimOutcome_Minted() when minted != null:
return minted(_that);case ReclaimOutcome_NotBroadcast() when notBroadcast != null:
return notBroadcast(_that);case ReclaimOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  nothingToReclaim,TResult Function( PlatformInt64 amountZat)?  minted,TResult Function( PlatformInt64 amountZat)?  notBroadcast,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim() when nothingToReclaim != null:
return nothingToReclaim();case ReclaimOutcome_Minted() when minted != null:
return minted(_that.amountZat);case ReclaimOutcome_NotBroadcast() when notBroadcast != null:
return notBroadcast(_that.amountZat);case ReclaimOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  nothingToReclaim,required TResult Function( PlatformInt64 amountZat)  minted,required TResult Function( PlatformInt64 amountZat)  notBroadcast,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim():
return nothingToReclaim();case ReclaimOutcome_Minted():
return minted(_that.amountZat);case ReclaimOutcome_NotBroadcast():
return notBroadcast(_that.amountZat);case ReclaimOutcome_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  nothingToReclaim,TResult? Function( PlatformInt64 amountZat)?  minted,TResult? Function( PlatformInt64 amountZat)?  notBroadcast,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case ReclaimOutcome_NothingToReclaim() when nothingToReclaim != null:
return nothingToReclaim();case ReclaimOutcome_Minted() when minted != null:
return minted(_that.amountZat);case ReclaimOutcome_NotBroadcast() when notBroadcast != null:
return notBroadcast(_that.amountZat);case ReclaimOutcome_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class ReclaimOutcome_NothingToReclaim extends ReclaimOutcome {
  const ReclaimOutcome_NothingToReclaim(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ReclaimOutcome_NothingToReclaim);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ReclaimOutcome.nothingToReclaim()';
}


}




/// @nodoc


class ReclaimOutcome_Minted extends ReclaimOutcome {
  const ReclaimOutcome_Minted({required this.amountZat}): super._();
  

/// The recovery-mint principal moved this call (a small fixed amount; host-rendered, never
/// logged). It is NOT a fee — it returns to the wallet; only the two transactions' fees cost.
 final  PlatformInt64 amountZat;

/// Create a copy of ReclaimOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ReclaimOutcome_MintedCopyWith<ReclaimOutcome_Minted> get copyWith => _$ReclaimOutcome_MintedCopyWithImpl<ReclaimOutcome_Minted>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ReclaimOutcome_Minted&&(identical(other.amountZat, amountZat) || other.amountZat == amountZat));
}


@override
int get hashCode => Object.hash(runtimeType,amountZat);

@override
String toString() {
  return 'ReclaimOutcome.minted(amountZat: $amountZat)';
}


}

/// @nodoc
abstract mixin class $ReclaimOutcome_MintedCopyWith<$Res> implements $ReclaimOutcomeCopyWith<$Res> {
  factory $ReclaimOutcome_MintedCopyWith(ReclaimOutcome_Minted value, $Res Function(ReclaimOutcome_Minted) _then) = _$ReclaimOutcome_MintedCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 amountZat
});




}
/// @nodoc
class _$ReclaimOutcome_MintedCopyWithImpl<$Res>
    implements $ReclaimOutcome_MintedCopyWith<$Res> {
  _$ReclaimOutcome_MintedCopyWithImpl(this._self, this._then);

  final ReclaimOutcome_Minted _self;
  final $Res Function(ReclaimOutcome_Minted) _then;

/// Create a copy of ReclaimOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? amountZat = null,}) {
  return _then(ReclaimOutcome_Minted(
amountZat: null == amountZat ? _self.amountZat : amountZat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class ReclaimOutcome_NotBroadcast extends ReclaimOutcome {
  const ReclaimOutcome_NotBroadcast({required this.amountZat}): super._();
  

/// The attempted mint principal (never spent — the transaction did not reach the network).
 final  PlatformInt64 amountZat;

/// Create a copy of ReclaimOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ReclaimOutcome_NotBroadcastCopyWith<ReclaimOutcome_NotBroadcast> get copyWith => _$ReclaimOutcome_NotBroadcastCopyWithImpl<ReclaimOutcome_NotBroadcast>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ReclaimOutcome_NotBroadcast&&(identical(other.amountZat, amountZat) || other.amountZat == amountZat));
}


@override
int get hashCode => Object.hash(runtimeType,amountZat);

@override
String toString() {
  return 'ReclaimOutcome.notBroadcast(amountZat: $amountZat)';
}


}

/// @nodoc
abstract mixin class $ReclaimOutcome_NotBroadcastCopyWith<$Res> implements $ReclaimOutcomeCopyWith<$Res> {
  factory $ReclaimOutcome_NotBroadcastCopyWith(ReclaimOutcome_NotBroadcast value, $Res Function(ReclaimOutcome_NotBroadcast) _then) = _$ReclaimOutcome_NotBroadcastCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 amountZat
});




}
/// @nodoc
class _$ReclaimOutcome_NotBroadcastCopyWithImpl<$Res>
    implements $ReclaimOutcome_NotBroadcastCopyWith<$Res> {
  _$ReclaimOutcome_NotBroadcastCopyWithImpl(this._self, this._then);

  final ReclaimOutcome_NotBroadcast _self;
  final $Res Function(ReclaimOutcome_NotBroadcast) _then;

/// Create a copy of ReclaimOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? amountZat = null,}) {
  return _then(ReclaimOutcome_NotBroadcast(
amountZat: null == amountZat ? _self.amountZat : amountZat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class ReclaimOutcome_Unknown extends ReclaimOutcome {
  const ReclaimOutcome_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ReclaimOutcome_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ReclaimOutcome.unknown()';
}


}




/// @nodoc
mixin _$SeverOutcome {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SeverOutcome()';
}


}

/// @nodoc
class $SeverOutcomeCopyWith<$Res>  {
$SeverOutcomeCopyWith(SeverOutcome _, $Res Function(SeverOutcome) __);
}


/// Adds pattern-matching-related methods to [SeverOutcome].
extension SeverOutcomePatterns on SeverOutcome {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SeverOutcome_Severed value)?  severed,TResult Function( SeverOutcome_SeveredUnproven value)?  severedUnproven,TResult Function( SeverOutcome_AlreadyGone value)?  alreadyGone,TResult Function( SeverOutcome_NotSevered value)?  notSevered,TResult Function( SeverOutcome_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SeverOutcome_Severed() when severed != null:
return severed(_that);case SeverOutcome_SeveredUnproven() when severedUnproven != null:
return severedUnproven(_that);case SeverOutcome_AlreadyGone() when alreadyGone != null:
return alreadyGone(_that);case SeverOutcome_NotSevered() when notSevered != null:
return notSevered(_that);case SeverOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SeverOutcome_Severed value)  severed,required TResult Function( SeverOutcome_SeveredUnproven value)  severedUnproven,required TResult Function( SeverOutcome_AlreadyGone value)  alreadyGone,required TResult Function( SeverOutcome_NotSevered value)  notSevered,required TResult Function( SeverOutcome_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SeverOutcome_Severed():
return severed(_that);case SeverOutcome_SeveredUnproven():
return severedUnproven(_that);case SeverOutcome_AlreadyGone():
return alreadyGone(_that);case SeverOutcome_NotSevered():
return notSevered(_that);case SeverOutcome_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SeverOutcome_Severed value)?  severed,TResult? Function( SeverOutcome_SeveredUnproven value)?  severedUnproven,TResult? Function( SeverOutcome_AlreadyGone value)?  alreadyGone,TResult? Function( SeverOutcome_NotSevered value)?  notSevered,TResult? Function( SeverOutcome_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SeverOutcome_Severed() when severed != null:
return severed(_that);case SeverOutcome_SeveredUnproven() when severedUnproven != null:
return severedUnproven(_that);case SeverOutcome_AlreadyGone() when alreadyGone != null:
return alreadyGone(_that);case SeverOutcome_NotSevered() when notSevered != null:
return notSevered(_that);case SeverOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int count)?  severed,TResult Function( UnprovenReason reason)?  severedUnproven,TResult Function()?  alreadyGone,TResult Function( NotSeveredCause cause)?  notSevered,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SeverOutcome_Severed() when severed != null:
return severed(_that.count);case SeverOutcome_SeveredUnproven() when severedUnproven != null:
return severedUnproven(_that.reason);case SeverOutcome_AlreadyGone() when alreadyGone != null:
return alreadyGone();case SeverOutcome_NotSevered() when notSevered != null:
return notSevered(_that.cause);case SeverOutcome_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int count)  severed,required TResult Function( UnprovenReason reason)  severedUnproven,required TResult Function()  alreadyGone,required TResult Function( NotSeveredCause cause)  notSevered,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SeverOutcome_Severed():
return severed(_that.count);case SeverOutcome_SeveredUnproven():
return severedUnproven(_that.reason);case SeverOutcome_AlreadyGone():
return alreadyGone();case SeverOutcome_NotSevered():
return notSevered(_that.cause);case SeverOutcome_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int count)?  severed,TResult? Function( UnprovenReason reason)?  severedUnproven,TResult? Function()?  alreadyGone,TResult? Function( NotSeveredCause cause)?  notSevered,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SeverOutcome_Severed() when severed != null:
return severed(_that.count);case SeverOutcome_SeveredUnproven() when severedUnproven != null:
return severedUnproven(_that.reason);case SeverOutcome_AlreadyGone() when alreadyGone != null:
return alreadyGone();case SeverOutcome_NotSevered() when notSevered != null:
return notSevered(_that.cause);case SeverOutcome_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SeverOutcome_Severed extends SeverOutcome {
  const SeverOutcome_Severed({required this.count}): super._();
  

/// Items the key store severed (saturates at the largest `int` the
/// bridge carries; always > 0).
 final  int count;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SeverOutcome_SeveredCopyWith<SeverOutcome_Severed> get copyWith => _$SeverOutcome_SeveredCopyWithImpl<SeverOutcome_Severed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome_Severed&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode => Object.hash(runtimeType,count);

@override
String toString() {
  return 'SeverOutcome.severed(count: $count)';
}


}

/// @nodoc
abstract mixin class $SeverOutcome_SeveredCopyWith<$Res> implements $SeverOutcomeCopyWith<$Res> {
  factory $SeverOutcome_SeveredCopyWith(SeverOutcome_Severed value, $Res Function(SeverOutcome_Severed) _then) = _$SeverOutcome_SeveredCopyWithImpl;
@useResult
$Res call({
 int count
});




}
/// @nodoc
class _$SeverOutcome_SeveredCopyWithImpl<$Res>
    implements $SeverOutcome_SeveredCopyWith<$Res> {
  _$SeverOutcome_SeveredCopyWithImpl(this._self, this._then);

  final SeverOutcome_Severed _self;
  final $Res Function(SeverOutcome_Severed) _then;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? count = null,}) {
  return _then(SeverOutcome_Severed(
count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SeverOutcome_SeveredUnproven extends SeverOutcome {
  const SeverOutcome_SeveredUnproven({required this.reason}): super._();
  

 final  UnprovenReason reason;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SeverOutcome_SeveredUnprovenCopyWith<SeverOutcome_SeveredUnproven> get copyWith => _$SeverOutcome_SeveredUnprovenCopyWithImpl<SeverOutcome_SeveredUnproven>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome_SeveredUnproven&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'SeverOutcome.severedUnproven(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SeverOutcome_SeveredUnprovenCopyWith<$Res> implements $SeverOutcomeCopyWith<$Res> {
  factory $SeverOutcome_SeveredUnprovenCopyWith(SeverOutcome_SeveredUnproven value, $Res Function(SeverOutcome_SeveredUnproven) _then) = _$SeverOutcome_SeveredUnprovenCopyWithImpl;
@useResult
$Res call({
 UnprovenReason reason
});




}
/// @nodoc
class _$SeverOutcome_SeveredUnprovenCopyWithImpl<$Res>
    implements $SeverOutcome_SeveredUnprovenCopyWith<$Res> {
  _$SeverOutcome_SeveredUnprovenCopyWithImpl(this._self, this._then);

  final SeverOutcome_SeveredUnproven _self;
  final $Res Function(SeverOutcome_SeveredUnproven) _then;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SeverOutcome_SeveredUnproven(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as UnprovenReason,
  ));
}


}

/// @nodoc


class SeverOutcome_AlreadyGone extends SeverOutcome {
  const SeverOutcome_AlreadyGone(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome_AlreadyGone);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SeverOutcome.alreadyGone()';
}


}




/// @nodoc


class SeverOutcome_NotSevered extends SeverOutcome {
  const SeverOutcome_NotSevered({required this.cause}): super._();
  

 final  NotSeveredCause cause;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SeverOutcome_NotSeveredCopyWith<SeverOutcome_NotSevered> get copyWith => _$SeverOutcome_NotSeveredCopyWithImpl<SeverOutcome_NotSevered>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome_NotSevered&&(identical(other.cause, cause) || other.cause == cause));
}


@override
int get hashCode => Object.hash(runtimeType,cause);

@override
String toString() {
  return 'SeverOutcome.notSevered(cause: $cause)';
}


}

/// @nodoc
abstract mixin class $SeverOutcome_NotSeveredCopyWith<$Res> implements $SeverOutcomeCopyWith<$Res> {
  factory $SeverOutcome_NotSeveredCopyWith(SeverOutcome_NotSevered value, $Res Function(SeverOutcome_NotSevered) _then) = _$SeverOutcome_NotSeveredCopyWithImpl;
@useResult
$Res call({
 NotSeveredCause cause
});




}
/// @nodoc
class _$SeverOutcome_NotSeveredCopyWithImpl<$Res>
    implements $SeverOutcome_NotSeveredCopyWith<$Res> {
  _$SeverOutcome_NotSeveredCopyWithImpl(this._self, this._then);

  final SeverOutcome_NotSevered _self;
  final $Res Function(SeverOutcome_NotSevered) _then;

/// Create a copy of SeverOutcome
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? cause = null,}) {
  return _then(SeverOutcome_NotSevered(
cause: null == cause ? _self.cause : cause // ignore: cast_nullable_to_non_nullable
as NotSeveredCause,
  ));
}


}

/// @nodoc


class SeverOutcome_Unknown extends SeverOutcome {
  const SeverOutcome_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SeverOutcome_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SeverOutcome.unknown()';
}


}




/// @nodoc
mixin _$SigningBlock {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SigningBlock);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SigningBlock()';
}


}

/// @nodoc
class $SigningBlockCopyWith<$Res>  {
$SigningBlockCopyWith(SigningBlock _, $Res Function(SigningBlock) __);
}


/// Adds pattern-matching-related methods to [SigningBlock].
extension SigningBlockPatterns on SigningBlock {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SigningBlock_NetworkUpgrade value)?  networkUpgrade,TResult Function( SigningBlock_GraceExpired value)?  graceExpired,TResult Function( SigningBlock_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade() when networkUpgrade != null:
return networkUpgrade(_that);case SigningBlock_GraceExpired() when graceExpired != null:
return graceExpired(_that);case SigningBlock_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SigningBlock_NetworkUpgrade value)  networkUpgrade,required TResult Function( SigningBlock_GraceExpired value)  graceExpired,required TResult Function( SigningBlock_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade():
return networkUpgrade(_that);case SigningBlock_GraceExpired():
return graceExpired(_that);case SigningBlock_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SigningBlock_NetworkUpgrade value)?  networkUpgrade,TResult? Function( SigningBlock_GraceExpired value)?  graceExpired,TResult? Function( SigningBlock_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade() when networkUpgrade != null:
return networkUpgrade(_that);case SigningBlock_GraceExpired() when graceExpired != null:
return graceExpired(_that);case SigningBlock_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  networkUpgrade,TResult Function( GraceExpiry by)?  graceExpired,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade() when networkUpgrade != null:
return networkUpgrade();case SigningBlock_GraceExpired() when graceExpired != null:
return graceExpired(_that.by);case SigningBlock_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  networkUpgrade,required TResult Function( GraceExpiry by)  graceExpired,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade():
return networkUpgrade();case SigningBlock_GraceExpired():
return graceExpired(_that.by);case SigningBlock_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  networkUpgrade,TResult? Function( GraceExpiry by)?  graceExpired,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SigningBlock_NetworkUpgrade() when networkUpgrade != null:
return networkUpgrade();case SigningBlock_GraceExpired() when graceExpired != null:
return graceExpired(_that.by);case SigningBlock_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SigningBlock_NetworkUpgrade extends SigningBlock {
  const SigningBlock_NetworkUpgrade(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SigningBlock_NetworkUpgrade);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SigningBlock.networkUpgrade()';
}


}




/// @nodoc


class SigningBlock_GraceExpired extends SigningBlock {
  const SigningBlock_GraceExpired({required this.by}): super._();
  

 final  GraceExpiry by;

/// Create a copy of SigningBlock
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SigningBlock_GraceExpiredCopyWith<SigningBlock_GraceExpired> get copyWith => _$SigningBlock_GraceExpiredCopyWithImpl<SigningBlock_GraceExpired>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SigningBlock_GraceExpired&&(identical(other.by, by) || other.by == by));
}


@override
int get hashCode => Object.hash(runtimeType,by);

@override
String toString() {
  return 'SigningBlock.graceExpired(by: $by)';
}


}

/// @nodoc
abstract mixin class $SigningBlock_GraceExpiredCopyWith<$Res> implements $SigningBlockCopyWith<$Res> {
  factory $SigningBlock_GraceExpiredCopyWith(SigningBlock_GraceExpired value, $Res Function(SigningBlock_GraceExpired) _then) = _$SigningBlock_GraceExpiredCopyWithImpl;
@useResult
$Res call({
 GraceExpiry by
});




}
/// @nodoc
class _$SigningBlock_GraceExpiredCopyWithImpl<$Res>
    implements $SigningBlock_GraceExpiredCopyWith<$Res> {
  _$SigningBlock_GraceExpiredCopyWithImpl(this._self, this._then);

  final SigningBlock_GraceExpired _self;
  final $Res Function(SigningBlock_GraceExpired) _then;

/// Create a copy of SigningBlock
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? by = null,}) {
  return _then(SigningBlock_GraceExpired(
by: null == by ? _self.by : by // ignore: cast_nullable_to_non_nullable
as GraceExpiry,
  ));
}


}

/// @nodoc


class SigningBlock_Unknown extends SigningBlock {
  const SigningBlock_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SigningBlock_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SigningBlock.unknown()';
}


}




/// @nodoc
mixin _$SyncStatus {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncStatus()';
}


}

/// @nodoc
class $SyncStatusCopyWith<$Res>  {
$SyncStatusCopyWith(SyncStatus _, $Res Function(SyncStatus) __);
}


/// Adds pattern-matching-related methods to [SyncStatus].
extension SyncStatusPatterns on SyncStatus {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncStatus_Idle value)?  idle,TResult Function( SyncStatus_Connecting value)?  connecting,TResult Function( SyncStatus_Scanning value)?  scanning,TResult Function( SyncStatus_UpToDate value)?  upToDate,TResult Function( SyncStatus_UpToDateLimited value)?  upToDateLimited,TResult Function( SyncStatus_UpToDateDegraded value)?  upToDateDegraded,TResult Function( SyncStatus_EndpointBehind value)?  endpointBehind,TResult Function( SyncStatus_UpToDateUnverified value)?  upToDateUnverified,TResult Function( SyncStatus_Stalled value)?  stalled,TResult Function( SyncStatus_Offline value)?  offline,TResult Function( SyncStatus_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncStatus_Idle() when idle != null:
return idle(_that);case SyncStatus_Connecting() when connecting != null:
return connecting(_that);case SyncStatus_Scanning() when scanning != null:
return scanning(_that);case SyncStatus_UpToDate() when upToDate != null:
return upToDate(_that);case SyncStatus_UpToDateLimited() when upToDateLimited != null:
return upToDateLimited(_that);case SyncStatus_UpToDateDegraded() when upToDateDegraded != null:
return upToDateDegraded(_that);case SyncStatus_EndpointBehind() when endpointBehind != null:
return endpointBehind(_that);case SyncStatus_UpToDateUnverified() when upToDateUnverified != null:
return upToDateUnverified(_that);case SyncStatus_Stalled() when stalled != null:
return stalled(_that);case SyncStatus_Offline() when offline != null:
return offline(_that);case SyncStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncStatus_Idle value)  idle,required TResult Function( SyncStatus_Connecting value)  connecting,required TResult Function( SyncStatus_Scanning value)  scanning,required TResult Function( SyncStatus_UpToDate value)  upToDate,required TResult Function( SyncStatus_UpToDateLimited value)  upToDateLimited,required TResult Function( SyncStatus_UpToDateDegraded value)  upToDateDegraded,required TResult Function( SyncStatus_EndpointBehind value)  endpointBehind,required TResult Function( SyncStatus_UpToDateUnverified value)  upToDateUnverified,required TResult Function( SyncStatus_Stalled value)  stalled,required TResult Function( SyncStatus_Offline value)  offline,required TResult Function( SyncStatus_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SyncStatus_Idle():
return idle(_that);case SyncStatus_Connecting():
return connecting(_that);case SyncStatus_Scanning():
return scanning(_that);case SyncStatus_UpToDate():
return upToDate(_that);case SyncStatus_UpToDateLimited():
return upToDateLimited(_that);case SyncStatus_UpToDateDegraded():
return upToDateDegraded(_that);case SyncStatus_EndpointBehind():
return endpointBehind(_that);case SyncStatus_UpToDateUnverified():
return upToDateUnverified(_that);case SyncStatus_Stalled():
return stalled(_that);case SyncStatus_Offline():
return offline(_that);case SyncStatus_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncStatus_Idle value)?  idle,TResult? Function( SyncStatus_Connecting value)?  connecting,TResult? Function( SyncStatus_Scanning value)?  scanning,TResult? Function( SyncStatus_UpToDate value)?  upToDate,TResult? Function( SyncStatus_UpToDateLimited value)?  upToDateLimited,TResult? Function( SyncStatus_UpToDateDegraded value)?  upToDateDegraded,TResult? Function( SyncStatus_EndpointBehind value)?  endpointBehind,TResult? Function( SyncStatus_UpToDateUnverified value)?  upToDateUnverified,TResult? Function( SyncStatus_Stalled value)?  stalled,TResult? Function( SyncStatus_Offline value)?  offline,TResult? Function( SyncStatus_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SyncStatus_Idle() when idle != null:
return idle(_that);case SyncStatus_Connecting() when connecting != null:
return connecting(_that);case SyncStatus_Scanning() when scanning != null:
return scanning(_that);case SyncStatus_UpToDate() when upToDate != null:
return upToDate(_that);case SyncStatus_UpToDateLimited() when upToDateLimited != null:
return upToDateLimited(_that);case SyncStatus_UpToDateDegraded() when upToDateDegraded != null:
return upToDateDegraded(_that);case SyncStatus_EndpointBehind() when endpointBehind != null:
return endpointBehind(_that);case SyncStatus_UpToDateUnverified() when upToDateUnverified != null:
return upToDateUnverified(_that);case SyncStatus_Stalled() when stalled != null:
return stalled(_that);case SyncStatus_Offline() when offline != null:
return offline(_that);case SyncStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  idle,TResult Function( double? torBootstrapPercent)?  connecting,TResult Function( int from,  int to,  double percent,  bool spendableReady,  bool rewound)?  scanning,TResult Function( int tip)?  upToDate,TResult Function( int tip)?  upToDateLimited,TResult Function( int tip,  PoolServiceReport pools)?  upToDateDegraded,TResult Function( int tip,  int newestKnown,  PoolServiceReport? pools)?  endpointBehind,TResult Function( int tip,  UnknownBranchGrace grace,  PoolServiceReport? pools,  bool streakReported)?  upToDateUnverified,TResult Function( StallReason reason)?  stalled,TResult Function( SyncStamp? lastSynced)?  offline,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncStatus_Idle() when idle != null:
return idle();case SyncStatus_Connecting() when connecting != null:
return connecting(_that.torBootstrapPercent);case SyncStatus_Scanning() when scanning != null:
return scanning(_that.from,_that.to,_that.percent,_that.spendableReady,_that.rewound);case SyncStatus_UpToDate() when upToDate != null:
return upToDate(_that.tip);case SyncStatus_UpToDateLimited() when upToDateLimited != null:
return upToDateLimited(_that.tip);case SyncStatus_UpToDateDegraded() when upToDateDegraded != null:
return upToDateDegraded(_that.tip,_that.pools);case SyncStatus_EndpointBehind() when endpointBehind != null:
return endpointBehind(_that.tip,_that.newestKnown,_that.pools);case SyncStatus_UpToDateUnverified() when upToDateUnverified != null:
return upToDateUnverified(_that.tip,_that.grace,_that.pools,_that.streakReported);case SyncStatus_Stalled() when stalled != null:
return stalled(_that.reason);case SyncStatus_Offline() when offline != null:
return offline(_that.lastSynced);case SyncStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  idle,required TResult Function( double? torBootstrapPercent)  connecting,required TResult Function( int from,  int to,  double percent,  bool spendableReady,  bool rewound)  scanning,required TResult Function( int tip)  upToDate,required TResult Function( int tip)  upToDateLimited,required TResult Function( int tip,  PoolServiceReport pools)  upToDateDegraded,required TResult Function( int tip,  int newestKnown,  PoolServiceReport? pools)  endpointBehind,required TResult Function( int tip,  UnknownBranchGrace grace,  PoolServiceReport? pools,  bool streakReported)  upToDateUnverified,required TResult Function( StallReason reason)  stalled,required TResult Function( SyncStamp? lastSynced)  offline,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SyncStatus_Idle():
return idle();case SyncStatus_Connecting():
return connecting(_that.torBootstrapPercent);case SyncStatus_Scanning():
return scanning(_that.from,_that.to,_that.percent,_that.spendableReady,_that.rewound);case SyncStatus_UpToDate():
return upToDate(_that.tip);case SyncStatus_UpToDateLimited():
return upToDateLimited(_that.tip);case SyncStatus_UpToDateDegraded():
return upToDateDegraded(_that.tip,_that.pools);case SyncStatus_EndpointBehind():
return endpointBehind(_that.tip,_that.newestKnown,_that.pools);case SyncStatus_UpToDateUnverified():
return upToDateUnverified(_that.tip,_that.grace,_that.pools,_that.streakReported);case SyncStatus_Stalled():
return stalled(_that.reason);case SyncStatus_Offline():
return offline(_that.lastSynced);case SyncStatus_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  idle,TResult? Function( double? torBootstrapPercent)?  connecting,TResult? Function( int from,  int to,  double percent,  bool spendableReady,  bool rewound)?  scanning,TResult? Function( int tip)?  upToDate,TResult? Function( int tip)?  upToDateLimited,TResult? Function( int tip,  PoolServiceReport pools)?  upToDateDegraded,TResult? Function( int tip,  int newestKnown,  PoolServiceReport? pools)?  endpointBehind,TResult? Function( int tip,  UnknownBranchGrace grace,  PoolServiceReport? pools,  bool streakReported)?  upToDateUnverified,TResult? Function( StallReason reason)?  stalled,TResult? Function( SyncStamp? lastSynced)?  offline,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SyncStatus_Idle() when idle != null:
return idle();case SyncStatus_Connecting() when connecting != null:
return connecting(_that.torBootstrapPercent);case SyncStatus_Scanning() when scanning != null:
return scanning(_that.from,_that.to,_that.percent,_that.spendableReady,_that.rewound);case SyncStatus_UpToDate() when upToDate != null:
return upToDate(_that.tip);case SyncStatus_UpToDateLimited() when upToDateLimited != null:
return upToDateLimited(_that.tip);case SyncStatus_UpToDateDegraded() when upToDateDegraded != null:
return upToDateDegraded(_that.tip,_that.pools);case SyncStatus_EndpointBehind() when endpointBehind != null:
return endpointBehind(_that.tip,_that.newestKnown,_that.pools);case SyncStatus_UpToDateUnverified() when upToDateUnverified != null:
return upToDateUnverified(_that.tip,_that.grace,_that.pools,_that.streakReported);case SyncStatus_Stalled() when stalled != null:
return stalled(_that.reason);case SyncStatus_Offline() when offline != null:
return offline(_that.lastSynced);case SyncStatus_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SyncStatus_Idle extends SyncStatus {
  const SyncStatus_Idle(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Idle);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncStatus.idle()';
}


}




/// @nodoc


class SyncStatus_Connecting extends SyncStatus {
  const SyncStatus_Connecting({this.torBootstrapPercent}): super._();
  

 final  double? torBootstrapPercent;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_ConnectingCopyWith<SyncStatus_Connecting> get copyWith => _$SyncStatus_ConnectingCopyWithImpl<SyncStatus_Connecting>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Connecting&&(identical(other.torBootstrapPercent, torBootstrapPercent) || other.torBootstrapPercent == torBootstrapPercent));
}


@override
int get hashCode => Object.hash(runtimeType,torBootstrapPercent);

@override
String toString() {
  return 'SyncStatus.connecting(torBootstrapPercent: $torBootstrapPercent)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_ConnectingCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_ConnectingCopyWith(SyncStatus_Connecting value, $Res Function(SyncStatus_Connecting) _then) = _$SyncStatus_ConnectingCopyWithImpl;
@useResult
$Res call({
 double? torBootstrapPercent
});




}
/// @nodoc
class _$SyncStatus_ConnectingCopyWithImpl<$Res>
    implements $SyncStatus_ConnectingCopyWith<$Res> {
  _$SyncStatus_ConnectingCopyWithImpl(this._self, this._then);

  final SyncStatus_Connecting _self;
  final $Res Function(SyncStatus_Connecting) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? torBootstrapPercent = freezed,}) {
  return _then(SyncStatus_Connecting(
torBootstrapPercent: freezed == torBootstrapPercent ? _self.torBootstrapPercent : torBootstrapPercent // ignore: cast_nullable_to_non_nullable
as double?,
  ));
}


}

/// @nodoc


class SyncStatus_Scanning extends SyncStatus {
  const SyncStatus_Scanning({required this.from, required this.to, required this.percent, required this.spendableReady, required this.rewound}): super._();
  

 final  int from;
 final  int to;
 final  double percent;
 final  bool spendableReady;
/// THIS pass has rewound at least once (a chain reorg un-scanned a
/// span). First-class so a host header/latch reacts to the rewind
/// explicitly instead of inferring it from a shrinking `to`; resets
/// with the pass (the next pass starts `false`).
 final  bool rewound;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_ScanningCopyWith<SyncStatus_Scanning> get copyWith => _$SyncStatus_ScanningCopyWithImpl<SyncStatus_Scanning>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Scanning&&(identical(other.from, from) || other.from == from)&&(identical(other.to, to) || other.to == to)&&(identical(other.percent, percent) || other.percent == percent)&&(identical(other.spendableReady, spendableReady) || other.spendableReady == spendableReady)&&(identical(other.rewound, rewound) || other.rewound == rewound));
}


@override
int get hashCode => Object.hash(runtimeType,from,to,percent,spendableReady,rewound);

@override
String toString() {
  return 'SyncStatus.scanning(from: $from, to: $to, percent: $percent, spendableReady: $spendableReady, rewound: $rewound)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_ScanningCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_ScanningCopyWith(SyncStatus_Scanning value, $Res Function(SyncStatus_Scanning) _then) = _$SyncStatus_ScanningCopyWithImpl;
@useResult
$Res call({
 int from, int to, double percent, bool spendableReady, bool rewound
});




}
/// @nodoc
class _$SyncStatus_ScanningCopyWithImpl<$Res>
    implements $SyncStatus_ScanningCopyWith<$Res> {
  _$SyncStatus_ScanningCopyWithImpl(this._self, this._then);

  final SyncStatus_Scanning _self;
  final $Res Function(SyncStatus_Scanning) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? from = null,Object? to = null,Object? percent = null,Object? spendableReady = null,Object? rewound = null,}) {
  return _then(SyncStatus_Scanning(
from: null == from ? _self.from : from // ignore: cast_nullable_to_non_nullable
as int,to: null == to ? _self.to : to // ignore: cast_nullable_to_non_nullable
as int,percent: null == percent ? _self.percent : percent // ignore: cast_nullable_to_non_nullable
as double,spendableReady: null == spendableReady ? _self.spendableReady : spendableReady // ignore: cast_nullable_to_non_nullable
as bool,rewound: null == rewound ? _self.rewound : rewound // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class SyncStatus_UpToDate extends SyncStatus {
  const SyncStatus_UpToDate({required this.tip}): super._();
  

 final  int tip;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_UpToDateCopyWith<SyncStatus_UpToDate> get copyWith => _$SyncStatus_UpToDateCopyWithImpl<SyncStatus_UpToDate>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_UpToDate&&(identical(other.tip, tip) || other.tip == tip));
}


@override
int get hashCode => Object.hash(runtimeType,tip);

@override
String toString() {
  return 'SyncStatus.upToDate(tip: $tip)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_UpToDateCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_UpToDateCopyWith(SyncStatus_UpToDate value, $Res Function(SyncStatus_UpToDate) _then) = _$SyncStatus_UpToDateCopyWithImpl;
@useResult
$Res call({
 int tip
});




}
/// @nodoc
class _$SyncStatus_UpToDateCopyWithImpl<$Res>
    implements $SyncStatus_UpToDateCopyWith<$Res> {
  _$SyncStatus_UpToDateCopyWithImpl(this._self, this._then);

  final SyncStatus_UpToDate _self;
  final $Res Function(SyncStatus_UpToDate) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tip = null,}) {
  return _then(SyncStatus_UpToDate(
tip: null == tip ? _self.tip : tip // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SyncStatus_UpToDateLimited extends SyncStatus {
  const SyncStatus_UpToDateLimited({required this.tip}): super._();
  

 final  int tip;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_UpToDateLimitedCopyWith<SyncStatus_UpToDateLimited> get copyWith => _$SyncStatus_UpToDateLimitedCopyWithImpl<SyncStatus_UpToDateLimited>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_UpToDateLimited&&(identical(other.tip, tip) || other.tip == tip));
}


@override
int get hashCode => Object.hash(runtimeType,tip);

@override
String toString() {
  return 'SyncStatus.upToDateLimited(tip: $tip)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_UpToDateLimitedCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_UpToDateLimitedCopyWith(SyncStatus_UpToDateLimited value, $Res Function(SyncStatus_UpToDateLimited) _then) = _$SyncStatus_UpToDateLimitedCopyWithImpl;
@useResult
$Res call({
 int tip
});




}
/// @nodoc
class _$SyncStatus_UpToDateLimitedCopyWithImpl<$Res>
    implements $SyncStatus_UpToDateLimitedCopyWith<$Res> {
  _$SyncStatus_UpToDateLimitedCopyWithImpl(this._self, this._then);

  final SyncStatus_UpToDateLimited _self;
  final $Res Function(SyncStatus_UpToDateLimited) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tip = null,}) {
  return _then(SyncStatus_UpToDateLimited(
tip: null == tip ? _self.tip : tip // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SyncStatus_UpToDateDegraded extends SyncStatus {
  const SyncStatus_UpToDateDegraded({required this.tip, required this.pools}): super._();
  

 final  int tip;
 final  PoolServiceReport pools;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_UpToDateDegradedCopyWith<SyncStatus_UpToDateDegraded> get copyWith => _$SyncStatus_UpToDateDegradedCopyWithImpl<SyncStatus_UpToDateDegraded>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_UpToDateDegraded&&(identical(other.tip, tip) || other.tip == tip)&&(identical(other.pools, pools) || other.pools == pools));
}


@override
int get hashCode => Object.hash(runtimeType,tip,pools);

@override
String toString() {
  return 'SyncStatus.upToDateDegraded(tip: $tip, pools: $pools)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_UpToDateDegradedCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_UpToDateDegradedCopyWith(SyncStatus_UpToDateDegraded value, $Res Function(SyncStatus_UpToDateDegraded) _then) = _$SyncStatus_UpToDateDegradedCopyWithImpl;
@useResult
$Res call({
 int tip, PoolServiceReport pools
});




}
/// @nodoc
class _$SyncStatus_UpToDateDegradedCopyWithImpl<$Res>
    implements $SyncStatus_UpToDateDegradedCopyWith<$Res> {
  _$SyncStatus_UpToDateDegradedCopyWithImpl(this._self, this._then);

  final SyncStatus_UpToDateDegraded _self;
  final $Res Function(SyncStatus_UpToDateDegraded) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tip = null,Object? pools = null,}) {
  return _then(SyncStatus_UpToDateDegraded(
tip: null == tip ? _self.tip : tip // ignore: cast_nullable_to_non_nullable
as int,pools: null == pools ? _self.pools : pools // ignore: cast_nullable_to_non_nullable
as PoolServiceReport,
  ));
}


}

/// @nodoc


class SyncStatus_EndpointBehind extends SyncStatus {
  const SyncStatus_EndpointBehind({required this.tip, required this.newestKnown, this.pools}): super._();
  

 final  int tip;
 final  int newestKnown;
 final  PoolServiceReport? pools;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_EndpointBehindCopyWith<SyncStatus_EndpointBehind> get copyWith => _$SyncStatus_EndpointBehindCopyWithImpl<SyncStatus_EndpointBehind>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_EndpointBehind&&(identical(other.tip, tip) || other.tip == tip)&&(identical(other.newestKnown, newestKnown) || other.newestKnown == newestKnown)&&(identical(other.pools, pools) || other.pools == pools));
}


@override
int get hashCode => Object.hash(runtimeType,tip,newestKnown,pools);

@override
String toString() {
  return 'SyncStatus.endpointBehind(tip: $tip, newestKnown: $newestKnown, pools: $pools)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_EndpointBehindCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_EndpointBehindCopyWith(SyncStatus_EndpointBehind value, $Res Function(SyncStatus_EndpointBehind) _then) = _$SyncStatus_EndpointBehindCopyWithImpl;
@useResult
$Res call({
 int tip, int newestKnown, PoolServiceReport? pools
});




}
/// @nodoc
class _$SyncStatus_EndpointBehindCopyWithImpl<$Res>
    implements $SyncStatus_EndpointBehindCopyWith<$Res> {
  _$SyncStatus_EndpointBehindCopyWithImpl(this._self, this._then);

  final SyncStatus_EndpointBehind _self;
  final $Res Function(SyncStatus_EndpointBehind) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tip = null,Object? newestKnown = null,Object? pools = freezed,}) {
  return _then(SyncStatus_EndpointBehind(
tip: null == tip ? _self.tip : tip // ignore: cast_nullable_to_non_nullable
as int,newestKnown: null == newestKnown ? _self.newestKnown : newestKnown // ignore: cast_nullable_to_non_nullable
as int,pools: freezed == pools ? _self.pools : pools // ignore: cast_nullable_to_non_nullable
as PoolServiceReport?,
  ));
}


}

/// @nodoc


class SyncStatus_UpToDateUnverified extends SyncStatus {
  const SyncStatus_UpToDateUnverified({required this.tip, required this.grace, this.pools, required this.streakReported}): super._();
  

 final  int tip;
 final  UnknownBranchGrace grace;
 final  PoolServiceReport? pools;
 final  bool streakReported;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_UpToDateUnverifiedCopyWith<SyncStatus_UpToDateUnverified> get copyWith => _$SyncStatus_UpToDateUnverifiedCopyWithImpl<SyncStatus_UpToDateUnverified>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_UpToDateUnverified&&(identical(other.tip, tip) || other.tip == tip)&&(identical(other.grace, grace) || other.grace == grace)&&(identical(other.pools, pools) || other.pools == pools)&&(identical(other.streakReported, streakReported) || other.streakReported == streakReported));
}


@override
int get hashCode => Object.hash(runtimeType,tip,grace,pools,streakReported);

@override
String toString() {
  return 'SyncStatus.upToDateUnverified(tip: $tip, grace: $grace, pools: $pools, streakReported: $streakReported)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_UpToDateUnverifiedCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_UpToDateUnverifiedCopyWith(SyncStatus_UpToDateUnverified value, $Res Function(SyncStatus_UpToDateUnverified) _then) = _$SyncStatus_UpToDateUnverifiedCopyWithImpl;
@useResult
$Res call({
 int tip, UnknownBranchGrace grace, PoolServiceReport? pools, bool streakReported
});


$UnknownBranchGraceCopyWith<$Res> get grace;

}
/// @nodoc
class _$SyncStatus_UpToDateUnverifiedCopyWithImpl<$Res>
    implements $SyncStatus_UpToDateUnverifiedCopyWith<$Res> {
  _$SyncStatus_UpToDateUnverifiedCopyWithImpl(this._self, this._then);

  final SyncStatus_UpToDateUnverified _self;
  final $Res Function(SyncStatus_UpToDateUnverified) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? tip = null,Object? grace = null,Object? pools = freezed,Object? streakReported = null,}) {
  return _then(SyncStatus_UpToDateUnverified(
tip: null == tip ? _self.tip : tip // ignore: cast_nullable_to_non_nullable
as int,grace: null == grace ? _self.grace : grace // ignore: cast_nullable_to_non_nullable
as UnknownBranchGrace,pools: freezed == pools ? _self.pools : pools // ignore: cast_nullable_to_non_nullable
as PoolServiceReport?,streakReported: null == streakReported ? _self.streakReported : streakReported // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$UnknownBranchGraceCopyWith<$Res> get grace {
  
  return $UnknownBranchGraceCopyWith<$Res>(_self.grace, (value) {
    return _then(_self.copyWith(grace: value));
  });
}
}

/// @nodoc


class SyncStatus_Stalled extends SyncStatus {
  const SyncStatus_Stalled({required this.reason}): super._();
  

 final  StallReason reason;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_StalledCopyWith<SyncStatus_Stalled> get copyWith => _$SyncStatus_StalledCopyWithImpl<SyncStatus_Stalled>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Stalled&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'SyncStatus.stalled(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_StalledCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_StalledCopyWith(SyncStatus_Stalled value, $Res Function(SyncStatus_Stalled) _then) = _$SyncStatus_StalledCopyWithImpl;
@useResult
$Res call({
 StallReason reason
});




}
/// @nodoc
class _$SyncStatus_StalledCopyWithImpl<$Res>
    implements $SyncStatus_StalledCopyWith<$Res> {
  _$SyncStatus_StalledCopyWithImpl(this._self, this._then);

  final SyncStatus_Stalled _self;
  final $Res Function(SyncStatus_Stalled) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SyncStatus_Stalled(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as StallReason,
  ));
}


}

/// @nodoc


class SyncStatus_Offline extends SyncStatus {
  const SyncStatus_Offline({this.lastSynced}): super._();
  

 final  SyncStamp? lastSynced;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncStatus_OfflineCopyWith<SyncStatus_Offline> get copyWith => _$SyncStatus_OfflineCopyWithImpl<SyncStatus_Offline>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Offline&&(identical(other.lastSynced, lastSynced) || other.lastSynced == lastSynced));
}


@override
int get hashCode => Object.hash(runtimeType,lastSynced);

@override
String toString() {
  return 'SyncStatus.offline(lastSynced: $lastSynced)';
}


}

/// @nodoc
abstract mixin class $SyncStatus_OfflineCopyWith<$Res> implements $SyncStatusCopyWith<$Res> {
  factory $SyncStatus_OfflineCopyWith(SyncStatus_Offline value, $Res Function(SyncStatus_Offline) _then) = _$SyncStatus_OfflineCopyWithImpl;
@useResult
$Res call({
 SyncStamp? lastSynced
});




}
/// @nodoc
class _$SyncStatus_OfflineCopyWithImpl<$Res>
    implements $SyncStatus_OfflineCopyWith<$Res> {
  _$SyncStatus_OfflineCopyWithImpl(this._self, this._then);

  final SyncStatus_Offline _self;
  final $Res Function(SyncStatus_Offline) _then;

/// Create a copy of SyncStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? lastSynced = freezed,}) {
  return _then(SyncStatus_Offline(
lastSynced: freezed == lastSynced ? _self.lastSynced : lastSynced // ignore: cast_nullable_to_non_nullable
as SyncStamp?,
  ));
}


}

/// @nodoc


class SyncStatus_Unknown extends SyncStatus {
  const SyncStatus_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncStatus_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncStatus.unknown()';
}


}




/// @nodoc
mixin _$TorRuntimeKind {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeKind()';
}


}

/// @nodoc
class $TorRuntimeKindCopyWith<$Res>  {
$TorRuntimeKindCopyWith(TorRuntimeKind _, $Res Function(TorRuntimeKind) __);
}


/// Adds pattern-matching-related methods to [TorRuntimeKind].
extension TorRuntimeKindPatterns on TorRuntimeKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TorRuntimeKind_ExternalSocks5 value)?  externalSocks5,TResult Function( TorRuntimeKind_Dialer value)?  dialer,TResult Function( TorRuntimeKind_HostDialer value)?  hostDialer,TResult Function( TorRuntimeKind_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that);case TorRuntimeKind_Dialer() when dialer != null:
return dialer(_that);case TorRuntimeKind_HostDialer() when hostDialer != null:
return hostDialer(_that);case TorRuntimeKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TorRuntimeKind_ExternalSocks5 value)  externalSocks5,required TResult Function( TorRuntimeKind_Dialer value)  dialer,required TResult Function( TorRuntimeKind_HostDialer value)  hostDialer,required TResult Function( TorRuntimeKind_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5():
return externalSocks5(_that);case TorRuntimeKind_Dialer():
return dialer(_that);case TorRuntimeKind_HostDialer():
return hostDialer(_that);case TorRuntimeKind_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TorRuntimeKind_ExternalSocks5 value)?  externalSocks5,TResult? Function( TorRuntimeKind_Dialer value)?  dialer,TResult? Function( TorRuntimeKind_HostDialer value)?  hostDialer,TResult? Function( TorRuntimeKind_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that);case TorRuntimeKind_Dialer() when dialer != null:
return dialer(_that);case TorRuntimeKind_HostDialer() when hostDialer != null:
return hostDialer(_that);case TorRuntimeKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  externalSocks5,TResult Function()?  dialer,TResult Function( String name,  IsolationSupport isolation,  TransportExposure exposure)?  hostDialer,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5() when externalSocks5 != null:
return externalSocks5();case TorRuntimeKind_Dialer() when dialer != null:
return dialer();case TorRuntimeKind_HostDialer() when hostDialer != null:
return hostDialer(_that.name,_that.isolation,_that.exposure);case TorRuntimeKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  externalSocks5,required TResult Function()  dialer,required TResult Function( String name,  IsolationSupport isolation,  TransportExposure exposure)  hostDialer,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5():
return externalSocks5();case TorRuntimeKind_Dialer():
return dialer();case TorRuntimeKind_HostDialer():
return hostDialer(_that.name,_that.isolation,_that.exposure);case TorRuntimeKind_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  externalSocks5,TResult? Function()?  dialer,TResult? Function( String name,  IsolationSupport isolation,  TransportExposure exposure)?  hostDialer,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case TorRuntimeKind_ExternalSocks5() when externalSocks5 != null:
return externalSocks5();case TorRuntimeKind_Dialer() when dialer != null:
return dialer();case TorRuntimeKind_HostDialer() when hostDialer != null:
return hostDialer(_that.name,_that.isolation,_that.exposure);case TorRuntimeKind_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class TorRuntimeKind_ExternalSocks5 extends TorRuntimeKind {
  const TorRuntimeKind_ExternalSocks5(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeKind_ExternalSocks5);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeKind.externalSocks5()';
}


}




/// @nodoc


class TorRuntimeKind_Dialer extends TorRuntimeKind {
  const TorRuntimeKind_Dialer(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeKind_Dialer);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeKind.dialer()';
}


}




/// @nodoc


class TorRuntimeKind_HostDialer extends TorRuntimeKind {
  const TorRuntimeKind_HostDialer({required this.name, required this.isolation, required this.exposure}): super._();
  

 final  String name;
 final  IsolationSupport isolation;
 final  TransportExposure exposure;

/// Create a copy of TorRuntimeKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorRuntimeKind_HostDialerCopyWith<TorRuntimeKind_HostDialer> get copyWith => _$TorRuntimeKind_HostDialerCopyWithImpl<TorRuntimeKind_HostDialer>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeKind_HostDialer&&(identical(other.name, name) || other.name == name)&&(identical(other.isolation, isolation) || other.isolation == isolation)&&(identical(other.exposure, exposure) || other.exposure == exposure));
}


@override
int get hashCode => Object.hash(runtimeType,name,isolation,exposure);

@override
String toString() {
  return 'TorRuntimeKind.hostDialer(name: $name, isolation: $isolation, exposure: $exposure)';
}


}

/// @nodoc
abstract mixin class $TorRuntimeKind_HostDialerCopyWith<$Res> implements $TorRuntimeKindCopyWith<$Res> {
  factory $TorRuntimeKind_HostDialerCopyWith(TorRuntimeKind_HostDialer value, $Res Function(TorRuntimeKind_HostDialer) _then) = _$TorRuntimeKind_HostDialerCopyWithImpl;
@useResult
$Res call({
 String name, IsolationSupport isolation, TransportExposure exposure
});




}
/// @nodoc
class _$TorRuntimeKind_HostDialerCopyWithImpl<$Res>
    implements $TorRuntimeKind_HostDialerCopyWith<$Res> {
  _$TorRuntimeKind_HostDialerCopyWithImpl(this._self, this._then);

  final TorRuntimeKind_HostDialer _self;
  final $Res Function(TorRuntimeKind_HostDialer) _then;

/// Create a copy of TorRuntimeKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? name = null,Object? isolation = null,Object? exposure = null,}) {
  return _then(TorRuntimeKind_HostDialer(
name: null == name ? _self.name : name // ignore: cast_nullable_to_non_nullable
as String,isolation: null == isolation ? _self.isolation : isolation // ignore: cast_nullable_to_non_nullable
as IsolationSupport,exposure: null == exposure ? _self.exposure : exposure // ignore: cast_nullable_to_non_nullable
as TransportExposure,
  ));
}


}

/// @nodoc


class TorRuntimeKind_Unknown extends TorRuntimeKind {
  const TorRuntimeKind_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeKind_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeKind.unknown()';
}


}




/// @nodoc
mixin _$TorState {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorState()';
}


}

/// @nodoc
class $TorStateCopyWith<$Res>  {
$TorStateCopyWith(TorState _, $Res Function(TorState) __);
}


/// Adds pattern-matching-related methods to [TorState].
extension TorStatePatterns on TorState {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TorState_Off value)?  off,TResult Function( TorState_Bootstrapping value)?  bootstrapping,TResult Function( TorState_Active value)?  active,TResult Function( TorState_FellBack value)?  fellBack,TResult Function( TorState_Unavailable value)?  unavailable,TResult Function( TorState_Unanswered value)?  unanswered,TResult Function( TorState_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TorState_Off() when off != null:
return off(_that);case TorState_Bootstrapping() when bootstrapping != null:
return bootstrapping(_that);case TorState_Active() when active != null:
return active(_that);case TorState_FellBack() when fellBack != null:
return fellBack(_that);case TorState_Unavailable() when unavailable != null:
return unavailable(_that);case TorState_Unanswered() when unanswered != null:
return unanswered(_that);case TorState_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TorState_Off value)  off,required TResult Function( TorState_Bootstrapping value)  bootstrapping,required TResult Function( TorState_Active value)  active,required TResult Function( TorState_FellBack value)  fellBack,required TResult Function( TorState_Unavailable value)  unavailable,required TResult Function( TorState_Unanswered value)  unanswered,required TResult Function( TorState_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case TorState_Off():
return off(_that);case TorState_Bootstrapping():
return bootstrapping(_that);case TorState_Active():
return active(_that);case TorState_FellBack():
return fellBack(_that);case TorState_Unavailable():
return unavailable(_that);case TorState_Unanswered():
return unanswered(_that);case TorState_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TorState_Off value)?  off,TResult? Function( TorState_Bootstrapping value)?  bootstrapping,TResult? Function( TorState_Active value)?  active,TResult? Function( TorState_FellBack value)?  fellBack,TResult? Function( TorState_Unavailable value)?  unavailable,TResult? Function( TorState_Unanswered value)?  unanswered,TResult? Function( TorState_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case TorState_Off() when off != null:
return off(_that);case TorState_Bootstrapping() when bootstrapping != null:
return bootstrapping(_that);case TorState_Active() when active != null:
return active(_that);case TorState_FellBack() when fellBack != null:
return fellBack(_that);case TorState_Unavailable() when unavailable != null:
return unavailable(_that);case TorState_Unanswered() when unanswered != null:
return unanswered(_that);case TorState_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  off,TResult Function( double? percent,  String? transport)?  bootstrapping,TResult Function( TorRuntimeKind runtime)?  active,TResult Function()?  fellBack,TResult Function( String? transport)?  unavailable,TResult Function( TorRuntimeKind runtime)?  unanswered,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TorState_Off() when off != null:
return off();case TorState_Bootstrapping() when bootstrapping != null:
return bootstrapping(_that.percent,_that.transport);case TorState_Active() when active != null:
return active(_that.runtime);case TorState_FellBack() when fellBack != null:
return fellBack();case TorState_Unavailable() when unavailable != null:
return unavailable(_that.transport);case TorState_Unanswered() when unanswered != null:
return unanswered(_that.runtime);case TorState_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  off,required TResult Function( double? percent,  String? transport)  bootstrapping,required TResult Function( TorRuntimeKind runtime)  active,required TResult Function()  fellBack,required TResult Function( String? transport)  unavailable,required TResult Function( TorRuntimeKind runtime)  unanswered,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case TorState_Off():
return off();case TorState_Bootstrapping():
return bootstrapping(_that.percent,_that.transport);case TorState_Active():
return active(_that.runtime);case TorState_FellBack():
return fellBack();case TorState_Unavailable():
return unavailable(_that.transport);case TorState_Unanswered():
return unanswered(_that.runtime);case TorState_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  off,TResult? Function( double? percent,  String? transport)?  bootstrapping,TResult? Function( TorRuntimeKind runtime)?  active,TResult? Function()?  fellBack,TResult? Function( String? transport)?  unavailable,TResult? Function( TorRuntimeKind runtime)?  unanswered,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case TorState_Off() when off != null:
return off();case TorState_Bootstrapping() when bootstrapping != null:
return bootstrapping(_that.percent,_that.transport);case TorState_Active() when active != null:
return active(_that.runtime);case TorState_FellBack() when fellBack != null:
return fellBack();case TorState_Unavailable() when unavailable != null:
return unavailable(_that.transport);case TorState_Unanswered() when unanswered != null:
return unanswered(_that.runtime);case TorState_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class TorState_Off extends TorState {
  const TorState_Off(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Off);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorState.off()';
}


}




/// @nodoc


class TorState_Bootstrapping extends TorState {
  const TorState_Bootstrapping({this.percent, this.transport}): super._();
  

 final  double? percent;
 final  String? transport;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorState_BootstrappingCopyWith<TorState_Bootstrapping> get copyWith => _$TorState_BootstrappingCopyWithImpl<TorState_Bootstrapping>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Bootstrapping&&(identical(other.percent, percent) || other.percent == percent)&&(identical(other.transport, transport) || other.transport == transport));
}


@override
int get hashCode => Object.hash(runtimeType,percent,transport);

@override
String toString() {
  return 'TorState.bootstrapping(percent: $percent, transport: $transport)';
}


}

/// @nodoc
abstract mixin class $TorState_BootstrappingCopyWith<$Res> implements $TorStateCopyWith<$Res> {
  factory $TorState_BootstrappingCopyWith(TorState_Bootstrapping value, $Res Function(TorState_Bootstrapping) _then) = _$TorState_BootstrappingCopyWithImpl;
@useResult
$Res call({
 double? percent, String? transport
});




}
/// @nodoc
class _$TorState_BootstrappingCopyWithImpl<$Res>
    implements $TorState_BootstrappingCopyWith<$Res> {
  _$TorState_BootstrappingCopyWithImpl(this._self, this._then);

  final TorState_Bootstrapping _self;
  final $Res Function(TorState_Bootstrapping) _then;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? percent = freezed,Object? transport = freezed,}) {
  return _then(TorState_Bootstrapping(
percent: freezed == percent ? _self.percent : percent // ignore: cast_nullable_to_non_nullable
as double?,transport: freezed == transport ? _self.transport : transport // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class TorState_Active extends TorState {
  const TorState_Active({required this.runtime}): super._();
  

 final  TorRuntimeKind runtime;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorState_ActiveCopyWith<TorState_Active> get copyWith => _$TorState_ActiveCopyWithImpl<TorState_Active>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Active&&(identical(other.runtime, runtime) || other.runtime == runtime));
}


@override
int get hashCode => Object.hash(runtimeType,runtime);

@override
String toString() {
  return 'TorState.active(runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class $TorState_ActiveCopyWith<$Res> implements $TorStateCopyWith<$Res> {
  factory $TorState_ActiveCopyWith(TorState_Active value, $Res Function(TorState_Active) _then) = _$TorState_ActiveCopyWithImpl;
@useResult
$Res call({
 TorRuntimeKind runtime
});


$TorRuntimeKindCopyWith<$Res> get runtime;

}
/// @nodoc
class _$TorState_ActiveCopyWithImpl<$Res>
    implements $TorState_ActiveCopyWith<$Res> {
  _$TorState_ActiveCopyWithImpl(this._self, this._then);

  final TorState_Active _self;
  final $Res Function(TorState_Active) _then;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? runtime = null,}) {
  return _then(TorState_Active(
runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as TorRuntimeKind,
  ));
}

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$TorRuntimeKindCopyWith<$Res> get runtime {
  
  return $TorRuntimeKindCopyWith<$Res>(_self.runtime, (value) {
    return _then(_self.copyWith(runtime: value));
  });
}
}

/// @nodoc


class TorState_FellBack extends TorState {
  const TorState_FellBack(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_FellBack);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorState.fellBack()';
}


}




/// @nodoc


class TorState_Unavailable extends TorState {
  const TorState_Unavailable({this.transport}): super._();
  

 final  String? transport;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorState_UnavailableCopyWith<TorState_Unavailable> get copyWith => _$TorState_UnavailableCopyWithImpl<TorState_Unavailable>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Unavailable&&(identical(other.transport, transport) || other.transport == transport));
}


@override
int get hashCode => Object.hash(runtimeType,transport);

@override
String toString() {
  return 'TorState.unavailable(transport: $transport)';
}


}

/// @nodoc
abstract mixin class $TorState_UnavailableCopyWith<$Res> implements $TorStateCopyWith<$Res> {
  factory $TorState_UnavailableCopyWith(TorState_Unavailable value, $Res Function(TorState_Unavailable) _then) = _$TorState_UnavailableCopyWithImpl;
@useResult
$Res call({
 String? transport
});




}
/// @nodoc
class _$TorState_UnavailableCopyWithImpl<$Res>
    implements $TorState_UnavailableCopyWith<$Res> {
  _$TorState_UnavailableCopyWithImpl(this._self, this._then);

  final TorState_Unavailable _self;
  final $Res Function(TorState_Unavailable) _then;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? transport = freezed,}) {
  return _then(TorState_Unavailable(
transport: freezed == transport ? _self.transport : transport // ignore: cast_nullable_to_non_nullable
as String?,
  ));
}


}

/// @nodoc


class TorState_Unanswered extends TorState {
  const TorState_Unanswered({required this.runtime}): super._();
  

 final  TorRuntimeKind runtime;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorState_UnansweredCopyWith<TorState_Unanswered> get copyWith => _$TorState_UnansweredCopyWithImpl<TorState_Unanswered>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Unanswered&&(identical(other.runtime, runtime) || other.runtime == runtime));
}


@override
int get hashCode => Object.hash(runtimeType,runtime);

@override
String toString() {
  return 'TorState.unanswered(runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class $TorState_UnansweredCopyWith<$Res> implements $TorStateCopyWith<$Res> {
  factory $TorState_UnansweredCopyWith(TorState_Unanswered value, $Res Function(TorState_Unanswered) _then) = _$TorState_UnansweredCopyWithImpl;
@useResult
$Res call({
 TorRuntimeKind runtime
});


$TorRuntimeKindCopyWith<$Res> get runtime;

}
/// @nodoc
class _$TorState_UnansweredCopyWithImpl<$Res>
    implements $TorState_UnansweredCopyWith<$Res> {
  _$TorState_UnansweredCopyWithImpl(this._self, this._then);

  final TorState_Unanswered _self;
  final $Res Function(TorState_Unanswered) _then;

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? runtime = null,}) {
  return _then(TorState_Unanswered(
runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as TorRuntimeKind,
  ));
}

/// Create a copy of TorState
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$TorRuntimeKindCopyWith<$Res> get runtime {
  
  return $TorRuntimeKindCopyWith<$Res>(_self.runtime, (value) {
    return _then(_self.copyWith(runtime: value));
  });
}
}

/// @nodoc


class TorState_Unknown extends TorState {
  const TorState_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorState_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorState.unknown()';
}


}




/// @nodoc
mixin _$TxStatus {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus()';
}


}

/// @nodoc
class $TxStatusCopyWith<$Res>  {
$TxStatusCopyWith(TxStatus _, $Res Function(TxStatus) __);
}


/// Adds pattern-matching-related methods to [TxStatus].
extension TxStatusPatterns on TxStatus {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TxStatus_Queued value)?  queued,TResult Function( TxStatus_Pending value)?  pending,TResult Function( TxStatus_Confirmed value)?  confirmed,TResult Function( TxStatus_Expired value)?  expired,TResult Function( TxStatus_Failed value)?  failed,TResult Function( TxStatus_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TxStatus_Queued() when queued != null:
return queued(_that);case TxStatus_Pending() when pending != null:
return pending(_that);case TxStatus_Confirmed() when confirmed != null:
return confirmed(_that);case TxStatus_Expired() when expired != null:
return expired(_that);case TxStatus_Failed() when failed != null:
return failed(_that);case TxStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TxStatus_Queued value)  queued,required TResult Function( TxStatus_Pending value)  pending,required TResult Function( TxStatus_Confirmed value)  confirmed,required TResult Function( TxStatus_Expired value)  expired,required TResult Function( TxStatus_Failed value)  failed,required TResult Function( TxStatus_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case TxStatus_Queued():
return queued(_that);case TxStatus_Pending():
return pending(_that);case TxStatus_Confirmed():
return confirmed(_that);case TxStatus_Expired():
return expired(_that);case TxStatus_Failed():
return failed(_that);case TxStatus_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TxStatus_Queued value)?  queued,TResult? Function( TxStatus_Pending value)?  pending,TResult? Function( TxStatus_Confirmed value)?  confirmed,TResult? Function( TxStatus_Expired value)?  expired,TResult? Function( TxStatus_Failed value)?  failed,TResult? Function( TxStatus_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case TxStatus_Queued() when queued != null:
return queued(_that);case TxStatus_Pending() when pending != null:
return pending(_that);case TxStatus_Confirmed() when confirmed != null:
return confirmed(_that);case TxStatus_Expired() when expired != null:
return expired(_that);case TxStatus_Failed() when failed != null:
return failed(_that);case TxStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  queued,TResult Function()?  pending,TResult Function( int depth)?  confirmed,TResult Function()?  expired,TResult Function()?  failed,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TxStatus_Queued() when queued != null:
return queued();case TxStatus_Pending() when pending != null:
return pending();case TxStatus_Confirmed() when confirmed != null:
return confirmed(_that.depth);case TxStatus_Expired() when expired != null:
return expired();case TxStatus_Failed() when failed != null:
return failed();case TxStatus_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  queued,required TResult Function()  pending,required TResult Function( int depth)  confirmed,required TResult Function()  expired,required TResult Function()  failed,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case TxStatus_Queued():
return queued();case TxStatus_Pending():
return pending();case TxStatus_Confirmed():
return confirmed(_that.depth);case TxStatus_Expired():
return expired();case TxStatus_Failed():
return failed();case TxStatus_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  queued,TResult? Function()?  pending,TResult? Function( int depth)?  confirmed,TResult? Function()?  expired,TResult? Function()?  failed,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case TxStatus_Queued() when queued != null:
return queued();case TxStatus_Pending() when pending != null:
return pending();case TxStatus_Confirmed() when confirmed != null:
return confirmed(_that.depth);case TxStatus_Expired() when expired != null:
return expired();case TxStatus_Failed() when failed != null:
return failed();case TxStatus_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class TxStatus_Queued extends TxStatus {
  const TxStatus_Queued(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Queued);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus.queued()';
}


}




/// @nodoc


class TxStatus_Pending extends TxStatus {
  const TxStatus_Pending(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Pending);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus.pending()';
}


}




/// @nodoc


class TxStatus_Confirmed extends TxStatus {
  const TxStatus_Confirmed({required this.depth}): super._();
  

 final  int depth;

/// Create a copy of TxStatus
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TxStatus_ConfirmedCopyWith<TxStatus_Confirmed> get copyWith => _$TxStatus_ConfirmedCopyWithImpl<TxStatus_Confirmed>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Confirmed&&(identical(other.depth, depth) || other.depth == depth));
}


@override
int get hashCode => Object.hash(runtimeType,depth);

@override
String toString() {
  return 'TxStatus.confirmed(depth: $depth)';
}


}

/// @nodoc
abstract mixin class $TxStatus_ConfirmedCopyWith<$Res> implements $TxStatusCopyWith<$Res> {
  factory $TxStatus_ConfirmedCopyWith(TxStatus_Confirmed value, $Res Function(TxStatus_Confirmed) _then) = _$TxStatus_ConfirmedCopyWithImpl;
@useResult
$Res call({
 int depth
});




}
/// @nodoc
class _$TxStatus_ConfirmedCopyWithImpl<$Res>
    implements $TxStatus_ConfirmedCopyWith<$Res> {
  _$TxStatus_ConfirmedCopyWithImpl(this._self, this._then);

  final TxStatus_Confirmed _self;
  final $Res Function(TxStatus_Confirmed) _then;

/// Create a copy of TxStatus
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? depth = null,}) {
  return _then(TxStatus_Confirmed(
depth: null == depth ? _self.depth : depth // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class TxStatus_Expired extends TxStatus {
  const TxStatus_Expired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Expired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus.expired()';
}


}




/// @nodoc


class TxStatus_Failed extends TxStatus {
  const TxStatus_Failed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Failed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus.failed()';
}


}




/// @nodoc


class TxStatus_Unknown extends TxStatus {
  const TxStatus_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxStatus_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxStatus.unknown()';
}


}




/// @nodoc
mixin _$TxSubmitResult {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxSubmitResult()';
}


}

/// @nodoc
class $TxSubmitResultCopyWith<$Res>  {
$TxSubmitResultCopyWith(TxSubmitResult _, $Res Function(TxSubmitResult) __);
}


/// Adds pattern-matching-related methods to [TxSubmitResult].
extension TxSubmitResultPatterns on TxSubmitResult {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TxSubmitResult_Success value)?  success,TResult Function( TxSubmitResult_GrpcFailure value)?  grpcFailure,TResult Function( TxSubmitResult_SubmitFailure value)?  submitFailure,TResult Function( TxSubmitResult_NotAttempted value)?  notAttempted,TResult Function( TxSubmitResult_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TxSubmitResult_Success() when success != null:
return success(_that);case TxSubmitResult_GrpcFailure() when grpcFailure != null:
return grpcFailure(_that);case TxSubmitResult_SubmitFailure() when submitFailure != null:
return submitFailure(_that);case TxSubmitResult_NotAttempted() when notAttempted != null:
return notAttempted(_that);case TxSubmitResult_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TxSubmitResult_Success value)  success,required TResult Function( TxSubmitResult_GrpcFailure value)  grpcFailure,required TResult Function( TxSubmitResult_SubmitFailure value)  submitFailure,required TResult Function( TxSubmitResult_NotAttempted value)  notAttempted,required TResult Function( TxSubmitResult_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case TxSubmitResult_Success():
return success(_that);case TxSubmitResult_GrpcFailure():
return grpcFailure(_that);case TxSubmitResult_SubmitFailure():
return submitFailure(_that);case TxSubmitResult_NotAttempted():
return notAttempted(_that);case TxSubmitResult_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TxSubmitResult_Success value)?  success,TResult? Function( TxSubmitResult_GrpcFailure value)?  grpcFailure,TResult? Function( TxSubmitResult_SubmitFailure value)?  submitFailure,TResult? Function( TxSubmitResult_NotAttempted value)?  notAttempted,TResult? Function( TxSubmitResult_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case TxSubmitResult_Success() when success != null:
return success(_that);case TxSubmitResult_GrpcFailure() when grpcFailure != null:
return grpcFailure(_that);case TxSubmitResult_SubmitFailure() when submitFailure != null:
return submitFailure(_that);case TxSubmitResult_NotAttempted() when notAttempted != null:
return notAttempted(_that);case TxSubmitResult_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String txidHex)?  success,TResult Function( String txidHex)?  grpcFailure,TResult Function( String txidHex,  int code)?  submitFailure,TResult Function( String txidHex)?  notAttempted,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TxSubmitResult_Success() when success != null:
return success(_that.txidHex);case TxSubmitResult_GrpcFailure() when grpcFailure != null:
return grpcFailure(_that.txidHex);case TxSubmitResult_SubmitFailure() when submitFailure != null:
return submitFailure(_that.txidHex,_that.code);case TxSubmitResult_NotAttempted() when notAttempted != null:
return notAttempted(_that.txidHex);case TxSubmitResult_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String txidHex)  success,required TResult Function( String txidHex)  grpcFailure,required TResult Function( String txidHex,  int code)  submitFailure,required TResult Function( String txidHex)  notAttempted,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case TxSubmitResult_Success():
return success(_that.txidHex);case TxSubmitResult_GrpcFailure():
return grpcFailure(_that.txidHex);case TxSubmitResult_SubmitFailure():
return submitFailure(_that.txidHex,_that.code);case TxSubmitResult_NotAttempted():
return notAttempted(_that.txidHex);case TxSubmitResult_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String txidHex)?  success,TResult? Function( String txidHex)?  grpcFailure,TResult? Function( String txidHex,  int code)?  submitFailure,TResult? Function( String txidHex)?  notAttempted,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case TxSubmitResult_Success() when success != null:
return success(_that.txidHex);case TxSubmitResult_GrpcFailure() when grpcFailure != null:
return grpcFailure(_that.txidHex);case TxSubmitResult_SubmitFailure() when submitFailure != null:
return submitFailure(_that.txidHex,_that.code);case TxSubmitResult_NotAttempted() when notAttempted != null:
return notAttempted(_that.txidHex);case TxSubmitResult_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class TxSubmitResult_Success extends TxSubmitResult {
  const TxSubmitResult_Success({required this.txidHex}): super._();
  

 final  String txidHex;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TxSubmitResult_SuccessCopyWith<TxSubmitResult_Success> get copyWith => _$TxSubmitResult_SuccessCopyWithImpl<TxSubmitResult_Success>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult_Success&&(identical(other.txidHex, txidHex) || other.txidHex == txidHex));
}


@override
int get hashCode => Object.hash(runtimeType,txidHex);

@override
String toString() {
  return 'TxSubmitResult.success(txidHex: $txidHex)';
}


}

/// @nodoc
abstract mixin class $TxSubmitResult_SuccessCopyWith<$Res> implements $TxSubmitResultCopyWith<$Res> {
  factory $TxSubmitResult_SuccessCopyWith(TxSubmitResult_Success value, $Res Function(TxSubmitResult_Success) _then) = _$TxSubmitResult_SuccessCopyWithImpl;
@useResult
$Res call({
 String txidHex
});




}
/// @nodoc
class _$TxSubmitResult_SuccessCopyWithImpl<$Res>
    implements $TxSubmitResult_SuccessCopyWith<$Res> {
  _$TxSubmitResult_SuccessCopyWithImpl(this._self, this._then);

  final TxSubmitResult_Success _self;
  final $Res Function(TxSubmitResult_Success) _then;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? txidHex = null,}) {
  return _then(TxSubmitResult_Success(
txidHex: null == txidHex ? _self.txidHex : txidHex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class TxSubmitResult_GrpcFailure extends TxSubmitResult {
  const TxSubmitResult_GrpcFailure({required this.txidHex}): super._();
  

 final  String txidHex;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TxSubmitResult_GrpcFailureCopyWith<TxSubmitResult_GrpcFailure> get copyWith => _$TxSubmitResult_GrpcFailureCopyWithImpl<TxSubmitResult_GrpcFailure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult_GrpcFailure&&(identical(other.txidHex, txidHex) || other.txidHex == txidHex));
}


@override
int get hashCode => Object.hash(runtimeType,txidHex);

@override
String toString() {
  return 'TxSubmitResult.grpcFailure(txidHex: $txidHex)';
}


}

/// @nodoc
abstract mixin class $TxSubmitResult_GrpcFailureCopyWith<$Res> implements $TxSubmitResultCopyWith<$Res> {
  factory $TxSubmitResult_GrpcFailureCopyWith(TxSubmitResult_GrpcFailure value, $Res Function(TxSubmitResult_GrpcFailure) _then) = _$TxSubmitResult_GrpcFailureCopyWithImpl;
@useResult
$Res call({
 String txidHex
});




}
/// @nodoc
class _$TxSubmitResult_GrpcFailureCopyWithImpl<$Res>
    implements $TxSubmitResult_GrpcFailureCopyWith<$Res> {
  _$TxSubmitResult_GrpcFailureCopyWithImpl(this._self, this._then);

  final TxSubmitResult_GrpcFailure _self;
  final $Res Function(TxSubmitResult_GrpcFailure) _then;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? txidHex = null,}) {
  return _then(TxSubmitResult_GrpcFailure(
txidHex: null == txidHex ? _self.txidHex : txidHex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class TxSubmitResult_SubmitFailure extends TxSubmitResult {
  const TxSubmitResult_SubmitFailure({required this.txidHex, required this.code}): super._();
  

 final  String txidHex;
 final  int code;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TxSubmitResult_SubmitFailureCopyWith<TxSubmitResult_SubmitFailure> get copyWith => _$TxSubmitResult_SubmitFailureCopyWithImpl<TxSubmitResult_SubmitFailure>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult_SubmitFailure&&(identical(other.txidHex, txidHex) || other.txidHex == txidHex)&&(identical(other.code, code) || other.code == code));
}


@override
int get hashCode => Object.hash(runtimeType,txidHex,code);

@override
String toString() {
  return 'TxSubmitResult.submitFailure(txidHex: $txidHex, code: $code)';
}


}

/// @nodoc
abstract mixin class $TxSubmitResult_SubmitFailureCopyWith<$Res> implements $TxSubmitResultCopyWith<$Res> {
  factory $TxSubmitResult_SubmitFailureCopyWith(TxSubmitResult_SubmitFailure value, $Res Function(TxSubmitResult_SubmitFailure) _then) = _$TxSubmitResult_SubmitFailureCopyWithImpl;
@useResult
$Res call({
 String txidHex, int code
});




}
/// @nodoc
class _$TxSubmitResult_SubmitFailureCopyWithImpl<$Res>
    implements $TxSubmitResult_SubmitFailureCopyWith<$Res> {
  _$TxSubmitResult_SubmitFailureCopyWithImpl(this._self, this._then);

  final TxSubmitResult_SubmitFailure _self;
  final $Res Function(TxSubmitResult_SubmitFailure) _then;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? txidHex = null,Object? code = null,}) {
  return _then(TxSubmitResult_SubmitFailure(
txidHex: null == txidHex ? _self.txidHex : txidHex // ignore: cast_nullable_to_non_nullable
as String,code: null == code ? _self.code : code // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class TxSubmitResult_NotAttempted extends TxSubmitResult {
  const TxSubmitResult_NotAttempted({required this.txidHex}): super._();
  

 final  String txidHex;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TxSubmitResult_NotAttemptedCopyWith<TxSubmitResult_NotAttempted> get copyWith => _$TxSubmitResult_NotAttemptedCopyWithImpl<TxSubmitResult_NotAttempted>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult_NotAttempted&&(identical(other.txidHex, txidHex) || other.txidHex == txidHex));
}


@override
int get hashCode => Object.hash(runtimeType,txidHex);

@override
String toString() {
  return 'TxSubmitResult.notAttempted(txidHex: $txidHex)';
}


}

/// @nodoc
abstract mixin class $TxSubmitResult_NotAttemptedCopyWith<$Res> implements $TxSubmitResultCopyWith<$Res> {
  factory $TxSubmitResult_NotAttemptedCopyWith(TxSubmitResult_NotAttempted value, $Res Function(TxSubmitResult_NotAttempted) _then) = _$TxSubmitResult_NotAttemptedCopyWithImpl;
@useResult
$Res call({
 String txidHex
});




}
/// @nodoc
class _$TxSubmitResult_NotAttemptedCopyWithImpl<$Res>
    implements $TxSubmitResult_NotAttemptedCopyWith<$Res> {
  _$TxSubmitResult_NotAttemptedCopyWithImpl(this._self, this._then);

  final TxSubmitResult_NotAttempted _self;
  final $Res Function(TxSubmitResult_NotAttempted) _then;

/// Create a copy of TxSubmitResult
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? txidHex = null,}) {
  return _then(TxSubmitResult_NotAttempted(
txidHex: null == txidHex ? _self.txidHex : txidHex // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class TxSubmitResult_Unknown extends TxSubmitResult {
  const TxSubmitResult_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TxSubmitResult_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TxSubmitResult.unknown()';
}


}




/// @nodoc
mixin _$UnknownBranchGrace {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UnknownBranchGrace);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UnknownBranchGrace()';
}


}

/// @nodoc
class $UnknownBranchGraceCopyWith<$Res>  {
$UnknownBranchGraceCopyWith(UnknownBranchGrace _, $Res Function(UnknownBranchGrace) __);
}


/// Adds pattern-matching-related methods to [UnknownBranchGrace].
extension UnknownBranchGracePatterns on UnknownBranchGrace {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( UnknownBranchGrace_Running value)?  running,TResult Function( UnknownBranchGrace_Ended value)?  ended,TResult Function( UnknownBranchGrace_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case UnknownBranchGrace_Running() when running != null:
return running(_that);case UnknownBranchGrace_Ended() when ended != null:
return ended(_that);case UnknownBranchGrace_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( UnknownBranchGrace_Running value)  running,required TResult Function( UnknownBranchGrace_Ended value)  ended,required TResult Function( UnknownBranchGrace_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case UnknownBranchGrace_Running():
return running(_that);case UnknownBranchGrace_Ended():
return ended(_that);case UnknownBranchGrace_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( UnknownBranchGrace_Running value)?  running,TResult? Function( UnknownBranchGrace_Ended value)?  ended,TResult? Function( UnknownBranchGrace_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case UnknownBranchGrace_Running() when running != null:
return running(_that);case UnknownBranchGrace_Ended() when ended != null:
return ended(_that);case UnknownBranchGrace_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int blocksLeft,  int? secsLeft)?  running,TResult Function( GraceExpiry by,  int? blocksSinceLastCurrent)?  ended,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case UnknownBranchGrace_Running() when running != null:
return running(_that.blocksLeft,_that.secsLeft);case UnknownBranchGrace_Ended() when ended != null:
return ended(_that.by,_that.blocksSinceLastCurrent);case UnknownBranchGrace_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int blocksLeft,  int? secsLeft)  running,required TResult Function( GraceExpiry by,  int? blocksSinceLastCurrent)  ended,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case UnknownBranchGrace_Running():
return running(_that.blocksLeft,_that.secsLeft);case UnknownBranchGrace_Ended():
return ended(_that.by,_that.blocksSinceLastCurrent);case UnknownBranchGrace_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int blocksLeft,  int? secsLeft)?  running,TResult? Function( GraceExpiry by,  int? blocksSinceLastCurrent)?  ended,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case UnknownBranchGrace_Running() when running != null:
return running(_that.blocksLeft,_that.secsLeft);case UnknownBranchGrace_Ended() when ended != null:
return ended(_that.by,_that.blocksSinceLastCurrent);case UnknownBranchGrace_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class UnknownBranchGrace_Running extends UnknownBranchGrace {
  const UnknownBranchGrace_Running({required this.blocksLeft, this.secsLeft}): super._();
  

 final  int blocksLeft;
 final  int? secsLeft;

/// Create a copy of UnknownBranchGrace
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UnknownBranchGrace_RunningCopyWith<UnknownBranchGrace_Running> get copyWith => _$UnknownBranchGrace_RunningCopyWithImpl<UnknownBranchGrace_Running>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UnknownBranchGrace_Running&&(identical(other.blocksLeft, blocksLeft) || other.blocksLeft == blocksLeft)&&(identical(other.secsLeft, secsLeft) || other.secsLeft == secsLeft));
}


@override
int get hashCode => Object.hash(runtimeType,blocksLeft,secsLeft);

@override
String toString() {
  return 'UnknownBranchGrace.running(blocksLeft: $blocksLeft, secsLeft: $secsLeft)';
}


}

/// @nodoc
abstract mixin class $UnknownBranchGrace_RunningCopyWith<$Res> implements $UnknownBranchGraceCopyWith<$Res> {
  factory $UnknownBranchGrace_RunningCopyWith(UnknownBranchGrace_Running value, $Res Function(UnknownBranchGrace_Running) _then) = _$UnknownBranchGrace_RunningCopyWithImpl;
@useResult
$Res call({
 int blocksLeft, int? secsLeft
});




}
/// @nodoc
class _$UnknownBranchGrace_RunningCopyWithImpl<$Res>
    implements $UnknownBranchGrace_RunningCopyWith<$Res> {
  _$UnknownBranchGrace_RunningCopyWithImpl(this._self, this._then);

  final UnknownBranchGrace_Running _self;
  final $Res Function(UnknownBranchGrace_Running) _then;

/// Create a copy of UnknownBranchGrace
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? blocksLeft = null,Object? secsLeft = freezed,}) {
  return _then(UnknownBranchGrace_Running(
blocksLeft: null == blocksLeft ? _self.blocksLeft : blocksLeft // ignore: cast_nullable_to_non_nullable
as int,secsLeft: freezed == secsLeft ? _self.secsLeft : secsLeft // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class UnknownBranchGrace_Ended extends UnknownBranchGrace {
  const UnknownBranchGrace_Ended({required this.by, this.blocksSinceLastCurrent}): super._();
  

 final  GraceExpiry by;
 final  int? blocksSinceLastCurrent;

/// Create a copy of UnknownBranchGrace
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$UnknownBranchGrace_EndedCopyWith<UnknownBranchGrace_Ended> get copyWith => _$UnknownBranchGrace_EndedCopyWithImpl<UnknownBranchGrace_Ended>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UnknownBranchGrace_Ended&&(identical(other.by, by) || other.by == by)&&(identical(other.blocksSinceLastCurrent, blocksSinceLastCurrent) || other.blocksSinceLastCurrent == blocksSinceLastCurrent));
}


@override
int get hashCode => Object.hash(runtimeType,by,blocksSinceLastCurrent);

@override
String toString() {
  return 'UnknownBranchGrace.ended(by: $by, blocksSinceLastCurrent: $blocksSinceLastCurrent)';
}


}

/// @nodoc
abstract mixin class $UnknownBranchGrace_EndedCopyWith<$Res> implements $UnknownBranchGraceCopyWith<$Res> {
  factory $UnknownBranchGrace_EndedCopyWith(UnknownBranchGrace_Ended value, $Res Function(UnknownBranchGrace_Ended) _then) = _$UnknownBranchGrace_EndedCopyWithImpl;
@useResult
$Res call({
 GraceExpiry by, int? blocksSinceLastCurrent
});




}
/// @nodoc
class _$UnknownBranchGrace_EndedCopyWithImpl<$Res>
    implements $UnknownBranchGrace_EndedCopyWith<$Res> {
  _$UnknownBranchGrace_EndedCopyWithImpl(this._self, this._then);

  final UnknownBranchGrace_Ended _self;
  final $Res Function(UnknownBranchGrace_Ended) _then;

/// Create a copy of UnknownBranchGrace
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? by = null,Object? blocksSinceLastCurrent = freezed,}) {
  return _then(UnknownBranchGrace_Ended(
by: null == by ? _self.by : by // ignore: cast_nullable_to_non_nullable
as GraceExpiry,blocksSinceLastCurrent: freezed == blocksSinceLastCurrent ? _self.blocksSinceLastCurrent : blocksSinceLastCurrent // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class UnknownBranchGrace_Unknown extends UnknownBranchGrace {
  const UnknownBranchGrace_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is UnknownBranchGrace_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'UnknownBranchGrace.unknown()';
}


}




// dart format on
