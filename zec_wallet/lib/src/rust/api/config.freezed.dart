// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'config.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$JitterPolicy {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JitterPolicy);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'JitterPolicy()';
}


}

/// @nodoc
class $JitterPolicyCopyWith<$Res>  {
$JitterPolicyCopyWith(JitterPolicy _, $Res Function(JitterPolicy) __);
}


/// Adds pattern-matching-related methods to [JitterPolicy].
extension JitterPolicyPatterns on JitterPolicy {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( JitterPolicy_None value)?  none,TResult Function( JitterPolicy_Uniform value)?  uniform,required TResult orElse(),}){
final _that = this;
switch (_that) {
case JitterPolicy_None() when none != null:
return none(_that);case JitterPolicy_Uniform() when uniform != null:
return uniform(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( JitterPolicy_None value)  none,required TResult Function( JitterPolicy_Uniform value)  uniform,}){
final _that = this;
switch (_that) {
case JitterPolicy_None():
return none(_that);case JitterPolicy_Uniform():
return uniform(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( JitterPolicy_None value)?  none,TResult? Function( JitterPolicy_Uniform value)?  uniform,}){
final _that = this;
switch (_that) {
case JitterPolicy_None() when none != null:
return none(_that);case JitterPolicy_Uniform() when uniform != null:
return uniform(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  none,TResult Function( int maxMs)?  uniform,required TResult orElse(),}) {final _that = this;
switch (_that) {
case JitterPolicy_None() when none != null:
return none();case JitterPolicy_Uniform() when uniform != null:
return uniform(_that.maxMs);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  none,required TResult Function( int maxMs)  uniform,}) {final _that = this;
switch (_that) {
case JitterPolicy_None():
return none();case JitterPolicy_Uniform():
return uniform(_that.maxMs);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  none,TResult? Function( int maxMs)?  uniform,}) {final _that = this;
switch (_that) {
case JitterPolicy_None() when none != null:
return none();case JitterPolicy_Uniform() when uniform != null:
return uniform(_that.maxMs);case _:
  return null;

}
}

}

/// @nodoc


class JitterPolicy_None extends JitterPolicy {
  const JitterPolicy_None(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JitterPolicy_None);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'JitterPolicy.none()';
}


}




/// @nodoc


class JitterPolicy_Uniform extends JitterPolicy {
  const JitterPolicy_Uniform({required this.maxMs}): super._();
  

 final  int maxMs;

/// Create a copy of JitterPolicy
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$JitterPolicy_UniformCopyWith<JitterPolicy_Uniform> get copyWith => _$JitterPolicy_UniformCopyWithImpl<JitterPolicy_Uniform>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is JitterPolicy_Uniform&&(identical(other.maxMs, maxMs) || other.maxMs == maxMs));
}


@override
int get hashCode => Object.hash(runtimeType,maxMs);

@override
String toString() {
  return 'JitterPolicy.uniform(maxMs: $maxMs)';
}


}

/// @nodoc
abstract mixin class $JitterPolicy_UniformCopyWith<$Res> implements $JitterPolicyCopyWith<$Res> {
  factory $JitterPolicy_UniformCopyWith(JitterPolicy_Uniform value, $Res Function(JitterPolicy_Uniform) _then) = _$JitterPolicy_UniformCopyWithImpl;
@useResult
$Res call({
 int maxMs
});




}
/// @nodoc
class _$JitterPolicy_UniformCopyWithImpl<$Res>
    implements $JitterPolicy_UniformCopyWith<$Res> {
  _$JitterPolicy_UniformCopyWithImpl(this._self, this._then);

  final JitterPolicy_Uniform _self;
  final $Res Function(JitterPolicy_Uniform) _then;

/// Create a copy of JitterPolicy
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? maxMs = null,}) {
  return _then(JitterPolicy_Uniform(
maxMs: null == maxMs ? _self.maxMs : maxMs // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc
mixin _$SyncServerChoice {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerChoice);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerChoice()';
}


}

/// @nodoc
class $SyncServerChoiceCopyWith<$Res>  {
$SyncServerChoiceCopyWith(SyncServerChoice _, $Res Function(SyncServerChoice) __);
}


/// Adds pattern-matching-related methods to [SyncServerChoice].
extension SyncServerChoicePatterns on SyncServerChoice {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncServerChoice_Predefined value)?  predefined,TResult Function( SyncServerChoice_Custom value)?  custom,TResult Function( SyncServerChoice_Default value)?  default_,TResult Function( SyncServerChoice_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncServerChoice_Predefined() when predefined != null:
return predefined(_that);case SyncServerChoice_Custom() when custom != null:
return custom(_that);case SyncServerChoice_Default() when default_ != null:
return default_(_that);case SyncServerChoice_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncServerChoice_Predefined value)  predefined,required TResult Function( SyncServerChoice_Custom value)  custom,required TResult Function( SyncServerChoice_Default value)  default_,required TResult Function( SyncServerChoice_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SyncServerChoice_Predefined():
return predefined(_that);case SyncServerChoice_Custom():
return custom(_that);case SyncServerChoice_Default():
return default_(_that);case SyncServerChoice_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncServerChoice_Predefined value)?  predefined,TResult? Function( SyncServerChoice_Custom value)?  custom,TResult? Function( SyncServerChoice_Default value)?  default_,TResult? Function( SyncServerChoice_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SyncServerChoice_Predefined() when predefined != null:
return predefined(_that);case SyncServerChoice_Custom() when custom != null:
return custom(_that);case SyncServerChoice_Default() when default_ != null:
return default_(_that);case SyncServerChoice_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String id)?  predefined,TResult Function( String url,  SyncServerKey? key)?  custom,TResult Function()?  default_,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncServerChoice_Predefined() when predefined != null:
return predefined(_that.id);case SyncServerChoice_Custom() when custom != null:
return custom(_that.url,_that.key);case SyncServerChoice_Default() when default_ != null:
return default_();case SyncServerChoice_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String id)  predefined,required TResult Function( String url,  SyncServerKey? key)  custom,required TResult Function()  default_,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SyncServerChoice_Predefined():
return predefined(_that.id);case SyncServerChoice_Custom():
return custom(_that.url,_that.key);case SyncServerChoice_Default():
return default_();case SyncServerChoice_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String id)?  predefined,TResult? Function( String url,  SyncServerKey? key)?  custom,TResult? Function()?  default_,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SyncServerChoice_Predefined() when predefined != null:
return predefined(_that.id);case SyncServerChoice_Custom() when custom != null:
return custom(_that.url,_that.key);case SyncServerChoice_Default() when default_ != null:
return default_();case SyncServerChoice_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SyncServerChoice_Predefined extends SyncServerChoice {
  const SyncServerChoice_Predefined({required this.id}): super._();
  

 final  String id;

/// Create a copy of SyncServerChoice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncServerChoice_PredefinedCopyWith<SyncServerChoice_Predefined> get copyWith => _$SyncServerChoice_PredefinedCopyWithImpl<SyncServerChoice_Predefined>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerChoice_Predefined&&(identical(other.id, id) || other.id == id));
}


@override
int get hashCode => Object.hash(runtimeType,id);

@override
String toString() {
  return 'SyncServerChoice.predefined(id: $id)';
}


}

/// @nodoc
abstract mixin class $SyncServerChoice_PredefinedCopyWith<$Res> implements $SyncServerChoiceCopyWith<$Res> {
  factory $SyncServerChoice_PredefinedCopyWith(SyncServerChoice_Predefined value, $Res Function(SyncServerChoice_Predefined) _then) = _$SyncServerChoice_PredefinedCopyWithImpl;
@useResult
$Res call({
 String id
});




}
/// @nodoc
class _$SyncServerChoice_PredefinedCopyWithImpl<$Res>
    implements $SyncServerChoice_PredefinedCopyWith<$Res> {
  _$SyncServerChoice_PredefinedCopyWithImpl(this._self, this._then);

  final SyncServerChoice_Predefined _self;
  final $Res Function(SyncServerChoice_Predefined) _then;

/// Create a copy of SyncServerChoice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = null,}) {
  return _then(SyncServerChoice_Predefined(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncServerChoice_Custom extends SyncServerChoice {
  const SyncServerChoice_Custom({required this.url, this.key}): super._();
  

 final  String url;
 final  SyncServerKey? key;

/// Create a copy of SyncServerChoice
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncServerChoice_CustomCopyWith<SyncServerChoice_Custom> get copyWith => _$SyncServerChoice_CustomCopyWithImpl<SyncServerChoice_Custom>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerChoice_Custom&&(identical(other.url, url) || other.url == url)&&(identical(other.key, key) || other.key == key));
}


@override
int get hashCode => Object.hash(runtimeType,url,key);

@override
String toString() {
  return 'SyncServerChoice.custom(url: $url, key: $key)';
}


}

/// @nodoc
abstract mixin class $SyncServerChoice_CustomCopyWith<$Res> implements $SyncServerChoiceCopyWith<$Res> {
  factory $SyncServerChoice_CustomCopyWith(SyncServerChoice_Custom value, $Res Function(SyncServerChoice_Custom) _then) = _$SyncServerChoice_CustomCopyWithImpl;
@useResult
$Res call({
 String url, SyncServerKey? key
});




}
/// @nodoc
class _$SyncServerChoice_CustomCopyWithImpl<$Res>
    implements $SyncServerChoice_CustomCopyWith<$Res> {
  _$SyncServerChoice_CustomCopyWithImpl(this._self, this._then);

  final SyncServerChoice_Custom _self;
  final $Res Function(SyncServerChoice_Custom) _then;

/// Create a copy of SyncServerChoice
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? url = null,Object? key = freezed,}) {
  return _then(SyncServerChoice_Custom(
url: null == url ? _self.url : url // ignore: cast_nullable_to_non_nullable
as String,key: freezed == key ? _self.key : key // ignore: cast_nullable_to_non_nullable
as SyncServerKey?,
  ));
}


}

/// @nodoc


class SyncServerChoice_Default extends SyncServerChoice {
  const SyncServerChoice_Default(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerChoice_Default);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerChoice.default_()';
}


}




/// @nodoc


class SyncServerChoice_Unknown extends SyncServerChoice {
  const SyncServerChoice_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerChoice_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerChoice.unknown()';
}


}




/// @nodoc
mixin _$SyncServerFallback {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerFallback);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerFallback()';
}


}

/// @nodoc
class $SyncServerFallbackCopyWith<$Res>  {
$SyncServerFallbackCopyWith(SyncServerFallback _, $Res Function(SyncServerFallback) __);
}


/// Adds pattern-matching-related methods to [SyncServerFallback].
extension SyncServerFallbackPatterns on SyncServerFallback {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SyncServerFallback_ChoiceNotOffered value)?  choiceNotOffered,TResult Function( SyncServerFallback_ChoiceUnreadable value)?  choiceUnreadable,TResult Function( SyncServerFallback_ChoiceRefusedByTransport value)?  choiceRefusedByTransport,TResult Function( SyncServerFallback_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered() when choiceNotOffered != null:
return choiceNotOffered(_that);case SyncServerFallback_ChoiceUnreadable() when choiceUnreadable != null:
return choiceUnreadable(_that);case SyncServerFallback_ChoiceRefusedByTransport() when choiceRefusedByTransport != null:
return choiceRefusedByTransport(_that);case SyncServerFallback_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SyncServerFallback_ChoiceNotOffered value)  choiceNotOffered,required TResult Function( SyncServerFallback_ChoiceUnreadable value)  choiceUnreadable,required TResult Function( SyncServerFallback_ChoiceRefusedByTransport value)  choiceRefusedByTransport,required TResult Function( SyncServerFallback_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered():
return choiceNotOffered(_that);case SyncServerFallback_ChoiceUnreadable():
return choiceUnreadable(_that);case SyncServerFallback_ChoiceRefusedByTransport():
return choiceRefusedByTransport(_that);case SyncServerFallback_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SyncServerFallback_ChoiceNotOffered value)?  choiceNotOffered,TResult? Function( SyncServerFallback_ChoiceUnreadable value)?  choiceUnreadable,TResult? Function( SyncServerFallback_ChoiceRefusedByTransport value)?  choiceRefusedByTransport,TResult? Function( SyncServerFallback_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered() when choiceNotOffered != null:
return choiceNotOffered(_that);case SyncServerFallback_ChoiceUnreadable() when choiceUnreadable != null:
return choiceUnreadable(_that);case SyncServerFallback_ChoiceRefusedByTransport() when choiceRefusedByTransport != null:
return choiceRefusedByTransport(_that);case SyncServerFallback_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String id)?  choiceNotOffered,TResult Function()?  choiceUnreadable,TResult Function()?  choiceRefusedByTransport,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered() when choiceNotOffered != null:
return choiceNotOffered(_that.id);case SyncServerFallback_ChoiceUnreadable() when choiceUnreadable != null:
return choiceUnreadable();case SyncServerFallback_ChoiceRefusedByTransport() when choiceRefusedByTransport != null:
return choiceRefusedByTransport();case SyncServerFallback_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String id)  choiceNotOffered,required TResult Function()  choiceUnreadable,required TResult Function()  choiceRefusedByTransport,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered():
return choiceNotOffered(_that.id);case SyncServerFallback_ChoiceUnreadable():
return choiceUnreadable();case SyncServerFallback_ChoiceRefusedByTransport():
return choiceRefusedByTransport();case SyncServerFallback_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String id)?  choiceNotOffered,TResult? Function()?  choiceUnreadable,TResult? Function()?  choiceRefusedByTransport,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SyncServerFallback_ChoiceNotOffered() when choiceNotOffered != null:
return choiceNotOffered(_that.id);case SyncServerFallback_ChoiceUnreadable() when choiceUnreadable != null:
return choiceUnreadable();case SyncServerFallback_ChoiceRefusedByTransport() when choiceRefusedByTransport != null:
return choiceRefusedByTransport();case SyncServerFallback_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SyncServerFallback_ChoiceNotOffered extends SyncServerFallback {
  const SyncServerFallback_ChoiceNotOffered({required this.id}): super._();
  

 final  String id;

/// Create a copy of SyncServerFallback
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SyncServerFallback_ChoiceNotOfferedCopyWith<SyncServerFallback_ChoiceNotOffered> get copyWith => _$SyncServerFallback_ChoiceNotOfferedCopyWithImpl<SyncServerFallback_ChoiceNotOffered>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerFallback_ChoiceNotOffered&&(identical(other.id, id) || other.id == id));
}


@override
int get hashCode => Object.hash(runtimeType,id);

@override
String toString() {
  return 'SyncServerFallback.choiceNotOffered(id: $id)';
}


}

/// @nodoc
abstract mixin class $SyncServerFallback_ChoiceNotOfferedCopyWith<$Res> implements $SyncServerFallbackCopyWith<$Res> {
  factory $SyncServerFallback_ChoiceNotOfferedCopyWith(SyncServerFallback_ChoiceNotOffered value, $Res Function(SyncServerFallback_ChoiceNotOffered) _then) = _$SyncServerFallback_ChoiceNotOfferedCopyWithImpl;
@useResult
$Res call({
 String id
});




}
/// @nodoc
class _$SyncServerFallback_ChoiceNotOfferedCopyWithImpl<$Res>
    implements $SyncServerFallback_ChoiceNotOfferedCopyWith<$Res> {
  _$SyncServerFallback_ChoiceNotOfferedCopyWithImpl(this._self, this._then);

  final SyncServerFallback_ChoiceNotOffered _self;
  final $Res Function(SyncServerFallback_ChoiceNotOffered) _then;

/// Create a copy of SyncServerFallback
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? id = null,}) {
  return _then(SyncServerFallback_ChoiceNotOffered(
id: null == id ? _self.id : id // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SyncServerFallback_ChoiceUnreadable extends SyncServerFallback {
  const SyncServerFallback_ChoiceUnreadable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerFallback_ChoiceUnreadable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerFallback.choiceUnreadable()';
}


}




/// @nodoc


class SyncServerFallback_ChoiceRefusedByTransport extends SyncServerFallback {
  const SyncServerFallback_ChoiceRefusedByTransport(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerFallback_ChoiceRefusedByTransport);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerFallback.choiceRefusedByTransport()';
}


}




/// @nodoc


class SyncServerFallback_Unknown extends SyncServerFallback {
  const SyncServerFallback_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SyncServerFallback_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SyncServerFallback.unknown()';
}


}




/// @nodoc
mixin _$TorPolicy {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorPolicy);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorPolicy()';
}


}

/// @nodoc
class $TorPolicyCopyWith<$Res>  {
$TorPolicyCopyWith(TorPolicy _, $Res Function(TorPolicy) __);
}


/// Adds pattern-matching-related methods to [TorPolicy].
extension TorPolicyPatterns on TorPolicy {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TorPolicy_Off value)?  off,TResult Function( TorPolicy_Preferred value)?  preferred,TResult Function( TorPolicy_Required value)?  required_,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TorPolicy_Off() when off != null:
return off(_that);case TorPolicy_Preferred() when preferred != null:
return preferred(_that);case TorPolicy_Required() when required_ != null:
return required_(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TorPolicy_Off value)  off,required TResult Function( TorPolicy_Preferred value)  preferred,required TResult Function( TorPolicy_Required value)  required_,}){
final _that = this;
switch (_that) {
case TorPolicy_Off():
return off(_that);case TorPolicy_Preferred():
return preferred(_that);case TorPolicy_Required():
return required_(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TorPolicy_Off value)?  off,TResult? Function( TorPolicy_Preferred value)?  preferred,TResult? Function( TorPolicy_Required value)?  required_,}){
final _that = this;
switch (_that) {
case TorPolicy_Off() when off != null:
return off(_that);case TorPolicy_Preferred() when preferred != null:
return preferred(_that);case TorPolicy_Required() when required_ != null:
return required_(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  off,TResult Function( TorRuntimeConfig runtime)?  preferred,TResult Function( TorRuntimeConfig runtime)?  required_,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TorPolicy_Off() when off != null:
return off();case TorPolicy_Preferred() when preferred != null:
return preferred(_that.runtime);case TorPolicy_Required() when required_ != null:
return required_(_that.runtime);case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  off,required TResult Function( TorRuntimeConfig runtime)  preferred,required TResult Function( TorRuntimeConfig runtime)  required_,}) {final _that = this;
switch (_that) {
case TorPolicy_Off():
return off();case TorPolicy_Preferred():
return preferred(_that.runtime);case TorPolicy_Required():
return required_(_that.runtime);}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  off,TResult? Function( TorRuntimeConfig runtime)?  preferred,TResult? Function( TorRuntimeConfig runtime)?  required_,}) {final _that = this;
switch (_that) {
case TorPolicy_Off() when off != null:
return off();case TorPolicy_Preferred() when preferred != null:
return preferred(_that.runtime);case TorPolicy_Required() when required_ != null:
return required_(_that.runtime);case _:
  return null;

}
}

}

/// @nodoc


class TorPolicy_Off extends TorPolicy {
  const TorPolicy_Off(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorPolicy_Off);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorPolicy.off()';
}


}




/// @nodoc


class TorPolicy_Preferred extends TorPolicy {
  const TorPolicy_Preferred({required this.runtime}): super._();
  

 final  TorRuntimeConfig runtime;

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorPolicy_PreferredCopyWith<TorPolicy_Preferred> get copyWith => _$TorPolicy_PreferredCopyWithImpl<TorPolicy_Preferred>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorPolicy_Preferred&&(identical(other.runtime, runtime) || other.runtime == runtime));
}


@override
int get hashCode => Object.hash(runtimeType,runtime);

@override
String toString() {
  return 'TorPolicy.preferred(runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class $TorPolicy_PreferredCopyWith<$Res> implements $TorPolicyCopyWith<$Res> {
  factory $TorPolicy_PreferredCopyWith(TorPolicy_Preferred value, $Res Function(TorPolicy_Preferred) _then) = _$TorPolicy_PreferredCopyWithImpl;
@useResult
$Res call({
 TorRuntimeConfig runtime
});


$TorRuntimeConfigCopyWith<$Res> get runtime;

}
/// @nodoc
class _$TorPolicy_PreferredCopyWithImpl<$Res>
    implements $TorPolicy_PreferredCopyWith<$Res> {
  _$TorPolicy_PreferredCopyWithImpl(this._self, this._then);

  final TorPolicy_Preferred _self;
  final $Res Function(TorPolicy_Preferred) _then;

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? runtime = null,}) {
  return _then(TorPolicy_Preferred(
runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as TorRuntimeConfig,
  ));
}

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$TorRuntimeConfigCopyWith<$Res> get runtime {
  
  return $TorRuntimeConfigCopyWith<$Res>(_self.runtime, (value) {
    return _then(_self.copyWith(runtime: value));
  });
}
}

/// @nodoc


class TorPolicy_Required extends TorPolicy {
  const TorPolicy_Required({required this.runtime}): super._();
  

 final  TorRuntimeConfig runtime;

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorPolicy_RequiredCopyWith<TorPolicy_Required> get copyWith => _$TorPolicy_RequiredCopyWithImpl<TorPolicy_Required>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorPolicy_Required&&(identical(other.runtime, runtime) || other.runtime == runtime));
}


@override
int get hashCode => Object.hash(runtimeType,runtime);

@override
String toString() {
  return 'TorPolicy.required_(runtime: $runtime)';
}


}

/// @nodoc
abstract mixin class $TorPolicy_RequiredCopyWith<$Res> implements $TorPolicyCopyWith<$Res> {
  factory $TorPolicy_RequiredCopyWith(TorPolicy_Required value, $Res Function(TorPolicy_Required) _then) = _$TorPolicy_RequiredCopyWithImpl;
@useResult
$Res call({
 TorRuntimeConfig runtime
});


$TorRuntimeConfigCopyWith<$Res> get runtime;

}
/// @nodoc
class _$TorPolicy_RequiredCopyWithImpl<$Res>
    implements $TorPolicy_RequiredCopyWith<$Res> {
  _$TorPolicy_RequiredCopyWithImpl(this._self, this._then);

  final TorPolicy_Required _self;
  final $Res Function(TorPolicy_Required) _then;

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? runtime = null,}) {
  return _then(TorPolicy_Required(
runtime: null == runtime ? _self.runtime : runtime // ignore: cast_nullable_to_non_nullable
as TorRuntimeConfig,
  ));
}

/// Create a copy of TorPolicy
/// with the given fields replaced by the non-null parameter values.
@override
@pragma('vm:prefer-inline')
$TorRuntimeConfigCopyWith<$Res> get runtime {
  
  return $TorRuntimeConfigCopyWith<$Res>(_self.runtime, (value) {
    return _then(_self.copyWith(runtime: value));
  });
}
}

/// @nodoc
mixin _$TorRuntimeConfig {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeConfig);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeConfig()';
}


}

/// @nodoc
class $TorRuntimeConfigCopyWith<$Res>  {
$TorRuntimeConfigCopyWith(TorRuntimeConfig _, $Res Function(TorRuntimeConfig) __);
}


/// Adds pattern-matching-related methods to [TorRuntimeConfig].
extension TorRuntimeConfigPatterns on TorRuntimeConfig {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( TorRuntimeConfig_ExternalSocks5 value)?  externalSocks5,TResult Function( TorRuntimeConfig_HostDialer value)?  hostDialer,required TResult orElse(),}){
final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that);case TorRuntimeConfig_HostDialer() when hostDialer != null:
return hostDialer(_that);case _:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( TorRuntimeConfig_ExternalSocks5 value)  externalSocks5,required TResult Function( TorRuntimeConfig_HostDialer value)  hostDialer,}){
final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5():
return externalSocks5(_that);case TorRuntimeConfig_HostDialer():
return hostDialer(_that);}
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( TorRuntimeConfig_ExternalSocks5 value)?  externalSocks5,TResult? Function( TorRuntimeConfig_HostDialer value)?  hostDialer,}){
final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that);case TorRuntimeConfig_HostDialer() when hostDialer != null:
return hostDialer(_that);case _:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( String addr)?  externalSocks5,TResult Function()?  hostDialer,required TResult orElse(),}) {final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that.addr);case TorRuntimeConfig_HostDialer() when hostDialer != null:
return hostDialer();case _:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( String addr)  externalSocks5,required TResult Function()  hostDialer,}) {final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5():
return externalSocks5(_that.addr);case TorRuntimeConfig_HostDialer():
return hostDialer();}
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( String addr)?  externalSocks5,TResult? Function()?  hostDialer,}) {final _that = this;
switch (_that) {
case TorRuntimeConfig_ExternalSocks5() when externalSocks5 != null:
return externalSocks5(_that.addr);case TorRuntimeConfig_HostDialer() when hostDialer != null:
return hostDialer();case _:
  return null;

}
}

}

/// @nodoc


class TorRuntimeConfig_ExternalSocks5 extends TorRuntimeConfig {
  const TorRuntimeConfig_ExternalSocks5({required this.addr}): super._();
  

 final  String addr;

/// Create a copy of TorRuntimeConfig
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$TorRuntimeConfig_ExternalSocks5CopyWith<TorRuntimeConfig_ExternalSocks5> get copyWith => _$TorRuntimeConfig_ExternalSocks5CopyWithImpl<TorRuntimeConfig_ExternalSocks5>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeConfig_ExternalSocks5&&(identical(other.addr, addr) || other.addr == addr));
}


@override
int get hashCode => Object.hash(runtimeType,addr);

@override
String toString() {
  return 'TorRuntimeConfig.externalSocks5(addr: $addr)';
}


}

/// @nodoc
abstract mixin class $TorRuntimeConfig_ExternalSocks5CopyWith<$Res> implements $TorRuntimeConfigCopyWith<$Res> {
  factory $TorRuntimeConfig_ExternalSocks5CopyWith(TorRuntimeConfig_ExternalSocks5 value, $Res Function(TorRuntimeConfig_ExternalSocks5) _then) = _$TorRuntimeConfig_ExternalSocks5CopyWithImpl;
@useResult
$Res call({
 String addr
});




}
/// @nodoc
class _$TorRuntimeConfig_ExternalSocks5CopyWithImpl<$Res>
    implements $TorRuntimeConfig_ExternalSocks5CopyWith<$Res> {
  _$TorRuntimeConfig_ExternalSocks5CopyWithImpl(this._self, this._then);

  final TorRuntimeConfig_ExternalSocks5 _self;
  final $Res Function(TorRuntimeConfig_ExternalSocks5) _then;

/// Create a copy of TorRuntimeConfig
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? addr = null,}) {
  return _then(TorRuntimeConfig_ExternalSocks5(
addr: null == addr ? _self.addr : addr // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class TorRuntimeConfig_HostDialer extends TorRuntimeConfig {
  const TorRuntimeConfig_HostDialer(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is TorRuntimeConfig_HostDialer);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'TorRuntimeConfig.hostDialer()';
}


}




// dart format on
