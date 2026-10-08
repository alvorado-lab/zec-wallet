// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'payments.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$ParsedMemo {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ParsedMemo()';
}


}

/// @nodoc
class $ParsedMemoCopyWith<$Res>  {
$ParsedMemoCopyWith(ParsedMemo _, $Res Function(ParsedMemo) __);
}


/// Adds pattern-matching-related methods to [ParsedMemo].
extension ParsedMemoPatterns on ParsedMemo {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( ParsedMemo_Empty value)?  empty,TResult Function( ParsedMemo_Text value)?  text,TResult Function( ParsedMemo_Arbitrary value)?  arbitrary,TResult Function( ParsedMemo_Reserved value)?  reserved,TResult Function( ParsedMemo_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case ParsedMemo_Empty() when empty != null:
return empty(_that);case ParsedMemo_Text() when text != null:
return text(_that);case ParsedMemo_Arbitrary() when arbitrary != null:
return arbitrary(_that);case ParsedMemo_Reserved() when reserved != null:
return reserved(_that);case ParsedMemo_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( ParsedMemo_Empty value)  empty,required TResult Function( ParsedMemo_Text value)  text,required TResult Function( ParsedMemo_Arbitrary value)  arbitrary,required TResult Function( ParsedMemo_Reserved value)  reserved,required TResult Function( ParsedMemo_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case ParsedMemo_Empty():
return empty(_that);case ParsedMemo_Text():
return text(_that);case ParsedMemo_Arbitrary():
return arbitrary(_that);case ParsedMemo_Reserved():
return reserved(_that);case ParsedMemo_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( ParsedMemo_Empty value)?  empty,TResult? Function( ParsedMemo_Text value)?  text,TResult? Function( ParsedMemo_Arbitrary value)?  arbitrary,TResult? Function( ParsedMemo_Reserved value)?  reserved,TResult? Function( ParsedMemo_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case ParsedMemo_Empty() when empty != null:
return empty(_that);case ParsedMemo_Text() when text != null:
return text(_that);case ParsedMemo_Arbitrary() when arbitrary != null:
return arbitrary(_that);case ParsedMemo_Reserved() when reserved != null:
return reserved(_that);case ParsedMemo_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function()?  empty,TResult Function( String text)?  text,TResult Function( int len)?  arbitrary,TResult Function( int len)?  reserved,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case ParsedMemo_Empty() when empty != null:
return empty();case ParsedMemo_Text() when text != null:
return text(_that.text);case ParsedMemo_Arbitrary() when arbitrary != null:
return arbitrary(_that.len);case ParsedMemo_Reserved() when reserved != null:
return reserved(_that.len);case ParsedMemo_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function()  empty,required TResult Function( String text)  text,required TResult Function( int len)  arbitrary,required TResult Function( int len)  reserved,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case ParsedMemo_Empty():
return empty();case ParsedMemo_Text():
return text(_that.text);case ParsedMemo_Arbitrary():
return arbitrary(_that.len);case ParsedMemo_Reserved():
return reserved(_that.len);case ParsedMemo_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function()?  empty,TResult? Function( String text)?  text,TResult? Function( int len)?  arbitrary,TResult? Function( int len)?  reserved,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case ParsedMemo_Empty() when empty != null:
return empty();case ParsedMemo_Text() when text != null:
return text(_that.text);case ParsedMemo_Arbitrary() when arbitrary != null:
return arbitrary(_that.len);case ParsedMemo_Reserved() when reserved != null:
return reserved(_that.len);case ParsedMemo_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class ParsedMemo_Empty extends ParsedMemo {
  const ParsedMemo_Empty(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo_Empty);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ParsedMemo.empty()';
}


}




/// @nodoc


class ParsedMemo_Text extends ParsedMemo {
  const ParsedMemo_Text({required this.text}): super._();
  

 final  String text;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ParsedMemo_TextCopyWith<ParsedMemo_Text> get copyWith => _$ParsedMemo_TextCopyWithImpl<ParsedMemo_Text>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo_Text&&(identical(other.text, text) || other.text == text));
}


@override
int get hashCode => Object.hash(runtimeType,text);

@override
String toString() {
  return 'ParsedMemo.text(text: $text)';
}


}

/// @nodoc
abstract mixin class $ParsedMemo_TextCopyWith<$Res> implements $ParsedMemoCopyWith<$Res> {
  factory $ParsedMemo_TextCopyWith(ParsedMemo_Text value, $Res Function(ParsedMemo_Text) _then) = _$ParsedMemo_TextCopyWithImpl;
@useResult
$Res call({
 String text
});




}
/// @nodoc
class _$ParsedMemo_TextCopyWithImpl<$Res>
    implements $ParsedMemo_TextCopyWith<$Res> {
  _$ParsedMemo_TextCopyWithImpl(this._self, this._then);

  final ParsedMemo_Text _self;
  final $Res Function(ParsedMemo_Text) _then;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? text = null,}) {
  return _then(ParsedMemo_Text(
text: null == text ? _self.text : text // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class ParsedMemo_Arbitrary extends ParsedMemo {
  const ParsedMemo_Arbitrary({required this.len}): super._();
  

 final  int len;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ParsedMemo_ArbitraryCopyWith<ParsedMemo_Arbitrary> get copyWith => _$ParsedMemo_ArbitraryCopyWithImpl<ParsedMemo_Arbitrary>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo_Arbitrary&&(identical(other.len, len) || other.len == len));
}


@override
int get hashCode => Object.hash(runtimeType,len);

@override
String toString() {
  return 'ParsedMemo.arbitrary(len: $len)';
}


}

/// @nodoc
abstract mixin class $ParsedMemo_ArbitraryCopyWith<$Res> implements $ParsedMemoCopyWith<$Res> {
  factory $ParsedMemo_ArbitraryCopyWith(ParsedMemo_Arbitrary value, $Res Function(ParsedMemo_Arbitrary) _then) = _$ParsedMemo_ArbitraryCopyWithImpl;
@useResult
$Res call({
 int len
});




}
/// @nodoc
class _$ParsedMemo_ArbitraryCopyWithImpl<$Res>
    implements $ParsedMemo_ArbitraryCopyWith<$Res> {
  _$ParsedMemo_ArbitraryCopyWithImpl(this._self, this._then);

  final ParsedMemo_Arbitrary _self;
  final $Res Function(ParsedMemo_Arbitrary) _then;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? len = null,}) {
  return _then(ParsedMemo_Arbitrary(
len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class ParsedMemo_Reserved extends ParsedMemo {
  const ParsedMemo_Reserved({required this.len}): super._();
  

 final  int len;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$ParsedMemo_ReservedCopyWith<ParsedMemo_Reserved> get copyWith => _$ParsedMemo_ReservedCopyWithImpl<ParsedMemo_Reserved>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo_Reserved&&(identical(other.len, len) || other.len == len));
}


@override
int get hashCode => Object.hash(runtimeType,len);

@override
String toString() {
  return 'ParsedMemo.reserved(len: $len)';
}


}

/// @nodoc
abstract mixin class $ParsedMemo_ReservedCopyWith<$Res> implements $ParsedMemoCopyWith<$Res> {
  factory $ParsedMemo_ReservedCopyWith(ParsedMemo_Reserved value, $Res Function(ParsedMemo_Reserved) _then) = _$ParsedMemo_ReservedCopyWithImpl;
@useResult
$Res call({
 int len
});




}
/// @nodoc
class _$ParsedMemo_ReservedCopyWithImpl<$Res>
    implements $ParsedMemo_ReservedCopyWith<$Res> {
  _$ParsedMemo_ReservedCopyWithImpl(this._self, this._then);

  final ParsedMemo_Reserved _self;
  final $Res Function(ParsedMemo_Reserved) _then;

/// Create a copy of ParsedMemo
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? len = null,}) {
  return _then(ParsedMemo_Reserved(
len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class ParsedMemo_Unknown extends ParsedMemo {
  const ParsedMemo_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is ParsedMemo_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'ParsedMemo.unknown()';
}


}




// dart format on
