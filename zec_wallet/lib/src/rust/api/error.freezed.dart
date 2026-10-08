// GENERATED CODE - DO NOT MODIFY BY HAND
// coverage:ignore-file
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'error.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

// dart format off
T _$identity<T>(T value) => value;
/// @nodoc
mixin _$SwapErrorKind {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind()';
}


}

/// @nodoc
class $SwapErrorKindCopyWith<$Res>  {
$SwapErrorKindCopyWith(SwapErrorKind _, $Res Function(SwapErrorKind) __);
}


/// Adds pattern-matching-related methods to [SwapErrorKind].
extension SwapErrorKindPatterns on SwapErrorKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( SwapErrorKind_SlippageToleranceTooHigh value)?  slippageToleranceTooHigh,TResult Function( SwapErrorKind_QuoteOutOfBounds value)?  quoteOutOfBounds,TResult Function( SwapErrorKind_QuoteExpired value)?  quoteExpired,TResult Function( SwapErrorKind_DestinationInvalid value)?  destinationInvalid,TResult Function( SwapErrorKind_RequestInvalid value)?  requestInvalid,TResult Function( SwapErrorKind_ProviderUnavailable value)?  providerUnavailable,TResult Function( SwapErrorKind_ProviderProtocol value)?  providerProtocol,TResult Function( SwapErrorKind_SwapDisabled value)?  swapDisabled,TResult Function( SwapErrorKind_DepositSendFailed value)?  depositSendFailed,TResult Function( SwapErrorKind_RefundAddressUnavailable value)?  refundAddressUnavailable,TResult Function( SwapErrorKind_DestinationAddressUnavailable value)?  destinationAddressUnavailable,TResult Function( SwapErrorKind_SwapStateUnavailable value)?  swapStateUnavailable,TResult Function( SwapErrorKind_SwapStateBusy value)?  swapStateBusy,TResult Function( SwapErrorKind_SwapAlreadyInFlight value)?  swapAlreadyInFlight,TResult Function( SwapErrorKind_WatchOnly value)?  watchOnly,TResult Function( SwapErrorKind_QuoteTermsDiffer value)?  quoteTermsDiffer,TResult Function( SwapErrorKind_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh() when slippageToleranceTooHigh != null:
return slippageToleranceTooHigh(_that);case SwapErrorKind_QuoteOutOfBounds() when quoteOutOfBounds != null:
return quoteOutOfBounds(_that);case SwapErrorKind_QuoteExpired() when quoteExpired != null:
return quoteExpired(_that);case SwapErrorKind_DestinationInvalid() when destinationInvalid != null:
return destinationInvalid(_that);case SwapErrorKind_RequestInvalid() when requestInvalid != null:
return requestInvalid(_that);case SwapErrorKind_ProviderUnavailable() when providerUnavailable != null:
return providerUnavailable(_that);case SwapErrorKind_ProviderProtocol() when providerProtocol != null:
return providerProtocol(_that);case SwapErrorKind_SwapDisabled() when swapDisabled != null:
return swapDisabled(_that);case SwapErrorKind_DepositSendFailed() when depositSendFailed != null:
return depositSendFailed(_that);case SwapErrorKind_RefundAddressUnavailable() when refundAddressUnavailable != null:
return refundAddressUnavailable(_that);case SwapErrorKind_DestinationAddressUnavailable() when destinationAddressUnavailable != null:
return destinationAddressUnavailable(_that);case SwapErrorKind_SwapStateUnavailable() when swapStateUnavailable != null:
return swapStateUnavailable(_that);case SwapErrorKind_SwapStateBusy() when swapStateBusy != null:
return swapStateBusy(_that);case SwapErrorKind_SwapAlreadyInFlight() when swapAlreadyInFlight != null:
return swapAlreadyInFlight(_that);case SwapErrorKind_WatchOnly() when watchOnly != null:
return watchOnly(_that);case SwapErrorKind_QuoteTermsDiffer() when quoteTermsDiffer != null:
return quoteTermsDiffer(_that);case SwapErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( SwapErrorKind_SlippageToleranceTooHigh value)  slippageToleranceTooHigh,required TResult Function( SwapErrorKind_QuoteOutOfBounds value)  quoteOutOfBounds,required TResult Function( SwapErrorKind_QuoteExpired value)  quoteExpired,required TResult Function( SwapErrorKind_DestinationInvalid value)  destinationInvalid,required TResult Function( SwapErrorKind_RequestInvalid value)  requestInvalid,required TResult Function( SwapErrorKind_ProviderUnavailable value)  providerUnavailable,required TResult Function( SwapErrorKind_ProviderProtocol value)  providerProtocol,required TResult Function( SwapErrorKind_SwapDisabled value)  swapDisabled,required TResult Function( SwapErrorKind_DepositSendFailed value)  depositSendFailed,required TResult Function( SwapErrorKind_RefundAddressUnavailable value)  refundAddressUnavailable,required TResult Function( SwapErrorKind_DestinationAddressUnavailable value)  destinationAddressUnavailable,required TResult Function( SwapErrorKind_SwapStateUnavailable value)  swapStateUnavailable,required TResult Function( SwapErrorKind_SwapStateBusy value)  swapStateBusy,required TResult Function( SwapErrorKind_SwapAlreadyInFlight value)  swapAlreadyInFlight,required TResult Function( SwapErrorKind_WatchOnly value)  watchOnly,required TResult Function( SwapErrorKind_QuoteTermsDiffer value)  quoteTermsDiffer,required TResult Function( SwapErrorKind_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh():
return slippageToleranceTooHigh(_that);case SwapErrorKind_QuoteOutOfBounds():
return quoteOutOfBounds(_that);case SwapErrorKind_QuoteExpired():
return quoteExpired(_that);case SwapErrorKind_DestinationInvalid():
return destinationInvalid(_that);case SwapErrorKind_RequestInvalid():
return requestInvalid(_that);case SwapErrorKind_ProviderUnavailable():
return providerUnavailable(_that);case SwapErrorKind_ProviderProtocol():
return providerProtocol(_that);case SwapErrorKind_SwapDisabled():
return swapDisabled(_that);case SwapErrorKind_DepositSendFailed():
return depositSendFailed(_that);case SwapErrorKind_RefundAddressUnavailable():
return refundAddressUnavailable(_that);case SwapErrorKind_DestinationAddressUnavailable():
return destinationAddressUnavailable(_that);case SwapErrorKind_SwapStateUnavailable():
return swapStateUnavailable(_that);case SwapErrorKind_SwapStateBusy():
return swapStateBusy(_that);case SwapErrorKind_SwapAlreadyInFlight():
return swapAlreadyInFlight(_that);case SwapErrorKind_WatchOnly():
return watchOnly(_that);case SwapErrorKind_QuoteTermsDiffer():
return quoteTermsDiffer(_that);case SwapErrorKind_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( SwapErrorKind_SlippageToleranceTooHigh value)?  slippageToleranceTooHigh,TResult? Function( SwapErrorKind_QuoteOutOfBounds value)?  quoteOutOfBounds,TResult? Function( SwapErrorKind_QuoteExpired value)?  quoteExpired,TResult? Function( SwapErrorKind_DestinationInvalid value)?  destinationInvalid,TResult? Function( SwapErrorKind_RequestInvalid value)?  requestInvalid,TResult? Function( SwapErrorKind_ProviderUnavailable value)?  providerUnavailable,TResult? Function( SwapErrorKind_ProviderProtocol value)?  providerProtocol,TResult? Function( SwapErrorKind_SwapDisabled value)?  swapDisabled,TResult? Function( SwapErrorKind_DepositSendFailed value)?  depositSendFailed,TResult? Function( SwapErrorKind_RefundAddressUnavailable value)?  refundAddressUnavailable,TResult? Function( SwapErrorKind_DestinationAddressUnavailable value)?  destinationAddressUnavailable,TResult? Function( SwapErrorKind_SwapStateUnavailable value)?  swapStateUnavailable,TResult? Function( SwapErrorKind_SwapStateBusy value)?  swapStateBusy,TResult? Function( SwapErrorKind_SwapAlreadyInFlight value)?  swapAlreadyInFlight,TResult? Function( SwapErrorKind_WatchOnly value)?  watchOnly,TResult? Function( SwapErrorKind_QuoteTermsDiffer value)?  quoteTermsDiffer,TResult? Function( SwapErrorKind_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh() when slippageToleranceTooHigh != null:
return slippageToleranceTooHigh(_that);case SwapErrorKind_QuoteOutOfBounds() when quoteOutOfBounds != null:
return quoteOutOfBounds(_that);case SwapErrorKind_QuoteExpired() when quoteExpired != null:
return quoteExpired(_that);case SwapErrorKind_DestinationInvalid() when destinationInvalid != null:
return destinationInvalid(_that);case SwapErrorKind_RequestInvalid() when requestInvalid != null:
return requestInvalid(_that);case SwapErrorKind_ProviderUnavailable() when providerUnavailable != null:
return providerUnavailable(_that);case SwapErrorKind_ProviderProtocol() when providerProtocol != null:
return providerProtocol(_that);case SwapErrorKind_SwapDisabled() when swapDisabled != null:
return swapDisabled(_that);case SwapErrorKind_DepositSendFailed() when depositSendFailed != null:
return depositSendFailed(_that);case SwapErrorKind_RefundAddressUnavailable() when refundAddressUnavailable != null:
return refundAddressUnavailable(_that);case SwapErrorKind_DestinationAddressUnavailable() when destinationAddressUnavailable != null:
return destinationAddressUnavailable(_that);case SwapErrorKind_SwapStateUnavailable() when swapStateUnavailable != null:
return swapStateUnavailable(_that);case SwapErrorKind_SwapStateBusy() when swapStateBusy != null:
return swapStateBusy(_that);case SwapErrorKind_SwapAlreadyInFlight() when swapAlreadyInFlight != null:
return swapAlreadyInFlight(_that);case SwapErrorKind_WatchOnly() when watchOnly != null:
return watchOnly(_that);case SwapErrorKind_QuoteTermsDiffer() when quoteTermsDiffer != null:
return quoteTermsDiffer(_that);case SwapErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( int requestedBps,  int maxBps)?  slippageToleranceTooHigh,TResult Function( QuoteBoundSide side)?  quoteOutOfBounds,TResult Function()?  quoteExpired,TResult Function( DestinationInvalidReason reason)?  destinationInvalid,TResult Function( String reason)?  requestInvalid,TResult Function()?  providerUnavailable,TResult Function( ProviderProtocolReason reason)?  providerProtocol,TResult Function()?  swapDisabled,TResult Function()?  depositSendFailed,TResult Function()?  refundAddressUnavailable,TResult Function()?  destinationAddressUnavailable,TResult Function()?  swapStateUnavailable,TResult Function()?  swapStateBusy,TResult Function()?  swapAlreadyInFlight,TResult Function()?  watchOnly,TResult Function()?  quoteTermsDiffer,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh() when slippageToleranceTooHigh != null:
return slippageToleranceTooHigh(_that.requestedBps,_that.maxBps);case SwapErrorKind_QuoteOutOfBounds() when quoteOutOfBounds != null:
return quoteOutOfBounds(_that.side);case SwapErrorKind_QuoteExpired() when quoteExpired != null:
return quoteExpired();case SwapErrorKind_DestinationInvalid() when destinationInvalid != null:
return destinationInvalid(_that.reason);case SwapErrorKind_RequestInvalid() when requestInvalid != null:
return requestInvalid(_that.reason);case SwapErrorKind_ProviderUnavailable() when providerUnavailable != null:
return providerUnavailable();case SwapErrorKind_ProviderProtocol() when providerProtocol != null:
return providerProtocol(_that.reason);case SwapErrorKind_SwapDisabled() when swapDisabled != null:
return swapDisabled();case SwapErrorKind_DepositSendFailed() when depositSendFailed != null:
return depositSendFailed();case SwapErrorKind_RefundAddressUnavailable() when refundAddressUnavailable != null:
return refundAddressUnavailable();case SwapErrorKind_DestinationAddressUnavailable() when destinationAddressUnavailable != null:
return destinationAddressUnavailable();case SwapErrorKind_SwapStateUnavailable() when swapStateUnavailable != null:
return swapStateUnavailable();case SwapErrorKind_SwapStateBusy() when swapStateBusy != null:
return swapStateBusy();case SwapErrorKind_SwapAlreadyInFlight() when swapAlreadyInFlight != null:
return swapAlreadyInFlight();case SwapErrorKind_WatchOnly() when watchOnly != null:
return watchOnly();case SwapErrorKind_QuoteTermsDiffer() when quoteTermsDiffer != null:
return quoteTermsDiffer();case SwapErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( int requestedBps,  int maxBps)  slippageToleranceTooHigh,required TResult Function( QuoteBoundSide side)  quoteOutOfBounds,required TResult Function()  quoteExpired,required TResult Function( DestinationInvalidReason reason)  destinationInvalid,required TResult Function( String reason)  requestInvalid,required TResult Function()  providerUnavailable,required TResult Function( ProviderProtocolReason reason)  providerProtocol,required TResult Function()  swapDisabled,required TResult Function()  depositSendFailed,required TResult Function()  refundAddressUnavailable,required TResult Function()  destinationAddressUnavailable,required TResult Function()  swapStateUnavailable,required TResult Function()  swapStateBusy,required TResult Function()  swapAlreadyInFlight,required TResult Function()  watchOnly,required TResult Function()  quoteTermsDiffer,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh():
return slippageToleranceTooHigh(_that.requestedBps,_that.maxBps);case SwapErrorKind_QuoteOutOfBounds():
return quoteOutOfBounds(_that.side);case SwapErrorKind_QuoteExpired():
return quoteExpired();case SwapErrorKind_DestinationInvalid():
return destinationInvalid(_that.reason);case SwapErrorKind_RequestInvalid():
return requestInvalid(_that.reason);case SwapErrorKind_ProviderUnavailable():
return providerUnavailable();case SwapErrorKind_ProviderProtocol():
return providerProtocol(_that.reason);case SwapErrorKind_SwapDisabled():
return swapDisabled();case SwapErrorKind_DepositSendFailed():
return depositSendFailed();case SwapErrorKind_RefundAddressUnavailable():
return refundAddressUnavailable();case SwapErrorKind_DestinationAddressUnavailable():
return destinationAddressUnavailable();case SwapErrorKind_SwapStateUnavailable():
return swapStateUnavailable();case SwapErrorKind_SwapStateBusy():
return swapStateBusy();case SwapErrorKind_SwapAlreadyInFlight():
return swapAlreadyInFlight();case SwapErrorKind_WatchOnly():
return watchOnly();case SwapErrorKind_QuoteTermsDiffer():
return quoteTermsDiffer();case SwapErrorKind_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( int requestedBps,  int maxBps)?  slippageToleranceTooHigh,TResult? Function( QuoteBoundSide side)?  quoteOutOfBounds,TResult? Function()?  quoteExpired,TResult? Function( DestinationInvalidReason reason)?  destinationInvalid,TResult? Function( String reason)?  requestInvalid,TResult? Function()?  providerUnavailable,TResult? Function( ProviderProtocolReason reason)?  providerProtocol,TResult? Function()?  swapDisabled,TResult? Function()?  depositSendFailed,TResult? Function()?  refundAddressUnavailable,TResult? Function()?  destinationAddressUnavailable,TResult? Function()?  swapStateUnavailable,TResult? Function()?  swapStateBusy,TResult? Function()?  swapAlreadyInFlight,TResult? Function()?  watchOnly,TResult? Function()?  quoteTermsDiffer,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case SwapErrorKind_SlippageToleranceTooHigh() when slippageToleranceTooHigh != null:
return slippageToleranceTooHigh(_that.requestedBps,_that.maxBps);case SwapErrorKind_QuoteOutOfBounds() when quoteOutOfBounds != null:
return quoteOutOfBounds(_that.side);case SwapErrorKind_QuoteExpired() when quoteExpired != null:
return quoteExpired();case SwapErrorKind_DestinationInvalid() when destinationInvalid != null:
return destinationInvalid(_that.reason);case SwapErrorKind_RequestInvalid() when requestInvalid != null:
return requestInvalid(_that.reason);case SwapErrorKind_ProviderUnavailable() when providerUnavailable != null:
return providerUnavailable();case SwapErrorKind_ProviderProtocol() when providerProtocol != null:
return providerProtocol(_that.reason);case SwapErrorKind_SwapDisabled() when swapDisabled != null:
return swapDisabled();case SwapErrorKind_DepositSendFailed() when depositSendFailed != null:
return depositSendFailed();case SwapErrorKind_RefundAddressUnavailable() when refundAddressUnavailable != null:
return refundAddressUnavailable();case SwapErrorKind_DestinationAddressUnavailable() when destinationAddressUnavailable != null:
return destinationAddressUnavailable();case SwapErrorKind_SwapStateUnavailable() when swapStateUnavailable != null:
return swapStateUnavailable();case SwapErrorKind_SwapStateBusy() when swapStateBusy != null:
return swapStateBusy();case SwapErrorKind_SwapAlreadyInFlight() when swapAlreadyInFlight != null:
return swapAlreadyInFlight();case SwapErrorKind_WatchOnly() when watchOnly != null:
return watchOnly();case SwapErrorKind_QuoteTermsDiffer() when quoteTermsDiffer != null:
return quoteTermsDiffer();case SwapErrorKind_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class SwapErrorKind_SlippageToleranceTooHigh extends SwapErrorKind {
  const SwapErrorKind_SlippageToleranceTooHigh({required this.requestedBps, required this.maxBps}): super._();
  

 final  int requestedBps;
 final  int maxBps;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapErrorKind_SlippageToleranceTooHighCopyWith<SwapErrorKind_SlippageToleranceTooHigh> get copyWith => _$SwapErrorKind_SlippageToleranceTooHighCopyWithImpl<SwapErrorKind_SlippageToleranceTooHigh>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_SlippageToleranceTooHigh&&(identical(other.requestedBps, requestedBps) || other.requestedBps == requestedBps)&&(identical(other.maxBps, maxBps) || other.maxBps == maxBps));
}


@override
int get hashCode => Object.hash(runtimeType,requestedBps,maxBps);

@override
String toString() {
  return 'SwapErrorKind.slippageToleranceTooHigh(requestedBps: $requestedBps, maxBps: $maxBps)';
}


}

/// @nodoc
abstract mixin class $SwapErrorKind_SlippageToleranceTooHighCopyWith<$Res> implements $SwapErrorKindCopyWith<$Res> {
  factory $SwapErrorKind_SlippageToleranceTooHighCopyWith(SwapErrorKind_SlippageToleranceTooHigh value, $Res Function(SwapErrorKind_SlippageToleranceTooHigh) _then) = _$SwapErrorKind_SlippageToleranceTooHighCopyWithImpl;
@useResult
$Res call({
 int requestedBps, int maxBps
});




}
/// @nodoc
class _$SwapErrorKind_SlippageToleranceTooHighCopyWithImpl<$Res>
    implements $SwapErrorKind_SlippageToleranceTooHighCopyWith<$Res> {
  _$SwapErrorKind_SlippageToleranceTooHighCopyWithImpl(this._self, this._then);

  final SwapErrorKind_SlippageToleranceTooHigh _self;
  final $Res Function(SwapErrorKind_SlippageToleranceTooHigh) _then;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? requestedBps = null,Object? maxBps = null,}) {
  return _then(SwapErrorKind_SlippageToleranceTooHigh(
requestedBps: null == requestedBps ? _self.requestedBps : requestedBps // ignore: cast_nullable_to_non_nullable
as int,maxBps: null == maxBps ? _self.maxBps : maxBps // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class SwapErrorKind_QuoteOutOfBounds extends SwapErrorKind {
  const SwapErrorKind_QuoteOutOfBounds({required this.side}): super._();
  

 final  QuoteBoundSide side;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapErrorKind_QuoteOutOfBoundsCopyWith<SwapErrorKind_QuoteOutOfBounds> get copyWith => _$SwapErrorKind_QuoteOutOfBoundsCopyWithImpl<SwapErrorKind_QuoteOutOfBounds>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_QuoteOutOfBounds&&(identical(other.side, side) || other.side == side));
}


@override
int get hashCode => Object.hash(runtimeType,side);

@override
String toString() {
  return 'SwapErrorKind.quoteOutOfBounds(side: $side)';
}


}

/// @nodoc
abstract mixin class $SwapErrorKind_QuoteOutOfBoundsCopyWith<$Res> implements $SwapErrorKindCopyWith<$Res> {
  factory $SwapErrorKind_QuoteOutOfBoundsCopyWith(SwapErrorKind_QuoteOutOfBounds value, $Res Function(SwapErrorKind_QuoteOutOfBounds) _then) = _$SwapErrorKind_QuoteOutOfBoundsCopyWithImpl;
@useResult
$Res call({
 QuoteBoundSide side
});




}
/// @nodoc
class _$SwapErrorKind_QuoteOutOfBoundsCopyWithImpl<$Res>
    implements $SwapErrorKind_QuoteOutOfBoundsCopyWith<$Res> {
  _$SwapErrorKind_QuoteOutOfBoundsCopyWithImpl(this._self, this._then);

  final SwapErrorKind_QuoteOutOfBounds _self;
  final $Res Function(SwapErrorKind_QuoteOutOfBounds) _then;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? side = null,}) {
  return _then(SwapErrorKind_QuoteOutOfBounds(
side: null == side ? _self.side : side // ignore: cast_nullable_to_non_nullable
as QuoteBoundSide,
  ));
}


}

/// @nodoc


class SwapErrorKind_QuoteExpired extends SwapErrorKind {
  const SwapErrorKind_QuoteExpired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_QuoteExpired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.quoteExpired()';
}


}




/// @nodoc


class SwapErrorKind_DestinationInvalid extends SwapErrorKind {
  const SwapErrorKind_DestinationInvalid({required this.reason}): super._();
  

 final  DestinationInvalidReason reason;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapErrorKind_DestinationInvalidCopyWith<SwapErrorKind_DestinationInvalid> get copyWith => _$SwapErrorKind_DestinationInvalidCopyWithImpl<SwapErrorKind_DestinationInvalid>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_DestinationInvalid&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'SwapErrorKind.destinationInvalid(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SwapErrorKind_DestinationInvalidCopyWith<$Res> implements $SwapErrorKindCopyWith<$Res> {
  factory $SwapErrorKind_DestinationInvalidCopyWith(SwapErrorKind_DestinationInvalid value, $Res Function(SwapErrorKind_DestinationInvalid) _then) = _$SwapErrorKind_DestinationInvalidCopyWithImpl;
@useResult
$Res call({
 DestinationInvalidReason reason
});




}
/// @nodoc
class _$SwapErrorKind_DestinationInvalidCopyWithImpl<$Res>
    implements $SwapErrorKind_DestinationInvalidCopyWith<$Res> {
  _$SwapErrorKind_DestinationInvalidCopyWithImpl(this._self, this._then);

  final SwapErrorKind_DestinationInvalid _self;
  final $Res Function(SwapErrorKind_DestinationInvalid) _then;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SwapErrorKind_DestinationInvalid(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as DestinationInvalidReason,
  ));
}


}

/// @nodoc


class SwapErrorKind_RequestInvalid extends SwapErrorKind {
  const SwapErrorKind_RequestInvalid({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapErrorKind_RequestInvalidCopyWith<SwapErrorKind_RequestInvalid> get copyWith => _$SwapErrorKind_RequestInvalidCopyWithImpl<SwapErrorKind_RequestInvalid>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_RequestInvalid&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'SwapErrorKind.requestInvalid(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SwapErrorKind_RequestInvalidCopyWith<$Res> implements $SwapErrorKindCopyWith<$Res> {
  factory $SwapErrorKind_RequestInvalidCopyWith(SwapErrorKind_RequestInvalid value, $Res Function(SwapErrorKind_RequestInvalid) _then) = _$SwapErrorKind_RequestInvalidCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$SwapErrorKind_RequestInvalidCopyWithImpl<$Res>
    implements $SwapErrorKind_RequestInvalidCopyWith<$Res> {
  _$SwapErrorKind_RequestInvalidCopyWithImpl(this._self, this._then);

  final SwapErrorKind_RequestInvalid _self;
  final $Res Function(SwapErrorKind_RequestInvalid) _then;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SwapErrorKind_RequestInvalid(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class SwapErrorKind_ProviderUnavailable extends SwapErrorKind {
  const SwapErrorKind_ProviderUnavailable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_ProviderUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.providerUnavailable()';
}


}




/// @nodoc


class SwapErrorKind_ProviderProtocol extends SwapErrorKind {
  const SwapErrorKind_ProviderProtocol({required this.reason}): super._();
  

 final  ProviderProtocolReason reason;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$SwapErrorKind_ProviderProtocolCopyWith<SwapErrorKind_ProviderProtocol> get copyWith => _$SwapErrorKind_ProviderProtocolCopyWithImpl<SwapErrorKind_ProviderProtocol>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_ProviderProtocol&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'SwapErrorKind.providerProtocol(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $SwapErrorKind_ProviderProtocolCopyWith<$Res> implements $SwapErrorKindCopyWith<$Res> {
  factory $SwapErrorKind_ProviderProtocolCopyWith(SwapErrorKind_ProviderProtocol value, $Res Function(SwapErrorKind_ProviderProtocol) _then) = _$SwapErrorKind_ProviderProtocolCopyWithImpl;
@useResult
$Res call({
 ProviderProtocolReason reason
});




}
/// @nodoc
class _$SwapErrorKind_ProviderProtocolCopyWithImpl<$Res>
    implements $SwapErrorKind_ProviderProtocolCopyWith<$Res> {
  _$SwapErrorKind_ProviderProtocolCopyWithImpl(this._self, this._then);

  final SwapErrorKind_ProviderProtocol _self;
  final $Res Function(SwapErrorKind_ProviderProtocol) _then;

/// Create a copy of SwapErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(SwapErrorKind_ProviderProtocol(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as ProviderProtocolReason,
  ));
}


}

/// @nodoc


class SwapErrorKind_SwapDisabled extends SwapErrorKind {
  const SwapErrorKind_SwapDisabled(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_SwapDisabled);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.swapDisabled()';
}


}




/// @nodoc


class SwapErrorKind_DepositSendFailed extends SwapErrorKind {
  const SwapErrorKind_DepositSendFailed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_DepositSendFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.depositSendFailed()';
}


}




/// @nodoc


class SwapErrorKind_RefundAddressUnavailable extends SwapErrorKind {
  const SwapErrorKind_RefundAddressUnavailable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_RefundAddressUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.refundAddressUnavailable()';
}


}




/// @nodoc


class SwapErrorKind_DestinationAddressUnavailable extends SwapErrorKind {
  const SwapErrorKind_DestinationAddressUnavailable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_DestinationAddressUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.destinationAddressUnavailable()';
}


}




/// @nodoc


class SwapErrorKind_SwapStateUnavailable extends SwapErrorKind {
  const SwapErrorKind_SwapStateUnavailable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_SwapStateUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.swapStateUnavailable()';
}


}




/// @nodoc


class SwapErrorKind_SwapStateBusy extends SwapErrorKind {
  const SwapErrorKind_SwapStateBusy(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_SwapStateBusy);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.swapStateBusy()';
}


}




/// @nodoc


class SwapErrorKind_SwapAlreadyInFlight extends SwapErrorKind {
  const SwapErrorKind_SwapAlreadyInFlight(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_SwapAlreadyInFlight);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.swapAlreadyInFlight()';
}


}




/// @nodoc


class SwapErrorKind_WatchOnly extends SwapErrorKind {
  const SwapErrorKind_WatchOnly(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_WatchOnly);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.watchOnly()';
}


}




/// @nodoc


class SwapErrorKind_QuoteTermsDiffer extends SwapErrorKind {
  const SwapErrorKind_QuoteTermsDiffer(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_QuoteTermsDiffer);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.quoteTermsDiffer()';
}


}




/// @nodoc


class SwapErrorKind_Unknown extends SwapErrorKind {
  const SwapErrorKind_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is SwapErrorKind_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'SwapErrorKind.unknown()';
}


}




/// @nodoc
mixin _$WalletErrorKind {





@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind()';
}


}

/// @nodoc
class $WalletErrorKindCopyWith<$Res>  {
$WalletErrorKindCopyWith(WalletErrorKind _, $Res Function(WalletErrorKind) __);
}


/// Adds pattern-matching-related methods to [WalletErrorKind].
extension WalletErrorKindPatterns on WalletErrorKind {
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

@optionalTypeArgs TResult maybeMap<TResult extends Object?>({TResult Function( WalletErrorKind_InvalidSeedLength value)?  invalidSeedLength,TResult Function( WalletErrorKind_InvalidMnemonic value)?  invalidMnemonic,TResult Function( WalletErrorKind_NoMnemonic value)?  noMnemonic,TResult Function( WalletErrorKind_SeedRequired value)?  seedRequired,TResult Function( WalletErrorKind_SeedMismatch value)?  seedMismatch,TResult Function( WalletErrorKind_KeyDerivation value)?  keyDerivation,TResult Function( WalletErrorKind_WatchOnly value)?  watchOnly,TResult Function( WalletErrorKind_InvalidViewingKey value)?  invalidViewingKey,TResult Function( WalletErrorKind_InvalidEndpoint value)?  invalidEndpoint,TResult Function( WalletErrorKind_BirthdayInFuture value)?  birthdayInFuture,TResult Function( WalletErrorKind_InvalidDbDir value)?  invalidDbDir,TResult Function( WalletErrorKind_InvalidEndpointAuth value)?  invalidEndpointAuth,TResult Function( WalletErrorKind_BroadcastJitterTooLong value)?  broadcastJitterTooLong,TResult Function( WalletErrorKind_AmountOutOfRange value)?  amountOutOfRange,TResult Function( WalletErrorKind_MemoTooLong value)?  memoTooLong,TResult Function( WalletErrorKind_ReservedMemoNotSendable value)?  reservedMemoNotSendable,TResult Function( WalletErrorKind_MemoRequiresShieldedRecipient value)?  memoRequiresShieldedRecipient,TResult Function( WalletErrorKind_AddressInvalid value)?  addressInvalid,TResult Function( WalletErrorKind_MemoInvalid value)?  memoInvalid,TResult Function( WalletErrorKind_MemoConflict value)?  memoConflict,TResult Function( WalletErrorKind_ZeroValuedTransparentOutput value)?  zeroValuedTransparentOutput,TResult Function( WalletErrorKind_PaymentUriInvalid value)?  paymentUriInvalid,TResult Function( WalletErrorKind_TxidInvalid value)?  txidInvalid,TResult Function( WalletErrorKind_MachineMemoScopeInvalid value)?  machineMemoScopeInvalid,TResult Function( WalletErrorKind_KeystoreUnavailable value)?  keystoreUnavailable,TResult Function( WalletErrorKind_KeystoreInconsistent value)?  keystoreInconsistent,TResult Function( WalletErrorKind_SealVersionUnsupported value)?  sealVersionUnsupported,TResult Function( WalletErrorKind_SealInvalid value)?  sealInvalid,TResult Function( WalletErrorKind_VaultAbsent value)?  vaultAbsent,TResult Function( WalletErrorKind_WrapArtifactInvalid value)?  wrapArtifactInvalid,TResult Function( WalletErrorKind_WrapVersionUnsupported value)?  wrapVersionUnsupported,TResult Function( WalletErrorKind_NotFound value)?  notFound,TResult Function( WalletErrorKind_NetworkMismatch value)?  networkMismatch,TResult Function( WalletErrorKind_ProvisioningIncomplete value)?  provisioningIncomplete,TResult Function( WalletErrorKind_WalletAlreadyExists value)?  walletAlreadyExists,TResult Function( WalletErrorKind_StoreCorrupt value)?  storeCorrupt,TResult Function( WalletErrorKind_DiskFull value)?  diskFull,TResult Function( WalletErrorKind_StoreBusy value)?  storeBusy,TResult Function( WalletErrorKind_WalletAlreadyOpen value)?  walletAlreadyOpen,TResult Function( WalletErrorKind_WalletBusy value)?  walletBusy,TResult Function( WalletErrorKind_InvalidState value)?  invalidState,TResult Function( WalletErrorKind_WalletOpen value)?  walletOpen,TResult Function( WalletErrorKind_SyncServerNotOffered value)?  syncServerNotOffered,TResult Function( WalletErrorKind_SyncServerUnreachable value)?  syncServerUnreachable,TResult Function( WalletErrorKind_WipeWithPendingSwap value)?  wipeWithPendingSwap,TResult Function( WalletErrorKind_RescanWithInFlightSend value)?  rescanWithInFlightSend,TResult Function( WalletErrorKind_SwapAddressCheckRefused value)?  swapAddressCheckRefused,TResult Function( WalletErrorKind_Sync value)?  sync_,TResult Function( WalletErrorKind_NetworkUpgradeUnsupported value)?  networkUpgradeUnsupported,TResult Function( WalletErrorKind_ConsensusGraceExpired value)?  consensusGraceExpired,TResult Function( WalletErrorKind_ConsensusNotEvaluated value)?  consensusNotEvaluated,TResult Function( WalletErrorKind_SyncRunning value)?  syncRunning,TResult Function( WalletErrorKind_InsufficientFunds value)?  insufficientFunds,TResult Function( WalletErrorKind_SendAmountRequired value)?  sendAmountRequired,TResult Function( WalletErrorKind_ProposalAlreadyUsed value)?  proposalAlreadyUsed,TResult Function( WalletErrorKind_ProposalStale value)?  proposalStale,TResult Function( WalletErrorKind_ProposeFailed value)?  proposeFailed,TResult Function( WalletErrorKind_ProposeTransient value)?  proposeTransient,TResult Function( WalletErrorKind_SignFailed value)?  signFailed,TResult Function( WalletErrorKind_QueuedSendStale value)?  queuedSendStale,TResult Function( WalletErrorKind_QueuedSendsFull value)?  queuedSendsFull,TResult Function( WalletErrorKind_TexSendLimitReached value)?  texSendLimitReached,TResult Function( WalletErrorKind_Io value)?  io,TResult Function( WalletErrorKind_Unknown value)?  unknown,required TResult orElse(),}){
final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength() when invalidSeedLength != null:
return invalidSeedLength(_that);case WalletErrorKind_InvalidMnemonic() when invalidMnemonic != null:
return invalidMnemonic(_that);case WalletErrorKind_NoMnemonic() when noMnemonic != null:
return noMnemonic(_that);case WalletErrorKind_SeedRequired() when seedRequired != null:
return seedRequired(_that);case WalletErrorKind_SeedMismatch() when seedMismatch != null:
return seedMismatch(_that);case WalletErrorKind_KeyDerivation() when keyDerivation != null:
return keyDerivation(_that);case WalletErrorKind_WatchOnly() when watchOnly != null:
return watchOnly(_that);case WalletErrorKind_InvalidViewingKey() when invalidViewingKey != null:
return invalidViewingKey(_that);case WalletErrorKind_InvalidEndpoint() when invalidEndpoint != null:
return invalidEndpoint(_that);case WalletErrorKind_BirthdayInFuture() when birthdayInFuture != null:
return birthdayInFuture(_that);case WalletErrorKind_InvalidDbDir() when invalidDbDir != null:
return invalidDbDir(_that);case WalletErrorKind_InvalidEndpointAuth() when invalidEndpointAuth != null:
return invalidEndpointAuth(_that);case WalletErrorKind_BroadcastJitterTooLong() when broadcastJitterTooLong != null:
return broadcastJitterTooLong(_that);case WalletErrorKind_AmountOutOfRange() when amountOutOfRange != null:
return amountOutOfRange(_that);case WalletErrorKind_MemoTooLong() when memoTooLong != null:
return memoTooLong(_that);case WalletErrorKind_ReservedMemoNotSendable() when reservedMemoNotSendable != null:
return reservedMemoNotSendable(_that);case WalletErrorKind_MemoRequiresShieldedRecipient() when memoRequiresShieldedRecipient != null:
return memoRequiresShieldedRecipient(_that);case WalletErrorKind_AddressInvalid() when addressInvalid != null:
return addressInvalid(_that);case WalletErrorKind_MemoInvalid() when memoInvalid != null:
return memoInvalid(_that);case WalletErrorKind_MemoConflict() when memoConflict != null:
return memoConflict(_that);case WalletErrorKind_ZeroValuedTransparentOutput() when zeroValuedTransparentOutput != null:
return zeroValuedTransparentOutput(_that);case WalletErrorKind_PaymentUriInvalid() when paymentUriInvalid != null:
return paymentUriInvalid(_that);case WalletErrorKind_TxidInvalid() when txidInvalid != null:
return txidInvalid(_that);case WalletErrorKind_MachineMemoScopeInvalid() when machineMemoScopeInvalid != null:
return machineMemoScopeInvalid(_that);case WalletErrorKind_KeystoreUnavailable() when keystoreUnavailable != null:
return keystoreUnavailable(_that);case WalletErrorKind_KeystoreInconsistent() when keystoreInconsistent != null:
return keystoreInconsistent(_that);case WalletErrorKind_SealVersionUnsupported() when sealVersionUnsupported != null:
return sealVersionUnsupported(_that);case WalletErrorKind_SealInvalid() when sealInvalid != null:
return sealInvalid(_that);case WalletErrorKind_VaultAbsent() when vaultAbsent != null:
return vaultAbsent(_that);case WalletErrorKind_WrapArtifactInvalid() when wrapArtifactInvalid != null:
return wrapArtifactInvalid(_that);case WalletErrorKind_WrapVersionUnsupported() when wrapVersionUnsupported != null:
return wrapVersionUnsupported(_that);case WalletErrorKind_NotFound() when notFound != null:
return notFound(_that);case WalletErrorKind_NetworkMismatch() when networkMismatch != null:
return networkMismatch(_that);case WalletErrorKind_ProvisioningIncomplete() when provisioningIncomplete != null:
return provisioningIncomplete(_that);case WalletErrorKind_WalletAlreadyExists() when walletAlreadyExists != null:
return walletAlreadyExists(_that);case WalletErrorKind_StoreCorrupt() when storeCorrupt != null:
return storeCorrupt(_that);case WalletErrorKind_DiskFull() when diskFull != null:
return diskFull(_that);case WalletErrorKind_StoreBusy() when storeBusy != null:
return storeBusy(_that);case WalletErrorKind_WalletAlreadyOpen() when walletAlreadyOpen != null:
return walletAlreadyOpen(_that);case WalletErrorKind_WalletBusy() when walletBusy != null:
return walletBusy(_that);case WalletErrorKind_InvalidState() when invalidState != null:
return invalidState(_that);case WalletErrorKind_WalletOpen() when walletOpen != null:
return walletOpen(_that);case WalletErrorKind_SyncServerNotOffered() when syncServerNotOffered != null:
return syncServerNotOffered(_that);case WalletErrorKind_SyncServerUnreachable() when syncServerUnreachable != null:
return syncServerUnreachable(_that);case WalletErrorKind_WipeWithPendingSwap() when wipeWithPendingSwap != null:
return wipeWithPendingSwap(_that);case WalletErrorKind_RescanWithInFlightSend() when rescanWithInFlightSend != null:
return rescanWithInFlightSend(_that);case WalletErrorKind_SwapAddressCheckRefused() when swapAddressCheckRefused != null:
return swapAddressCheckRefused(_that);case WalletErrorKind_Sync() when sync_ != null:
return sync_(_that);case WalletErrorKind_NetworkUpgradeUnsupported() when networkUpgradeUnsupported != null:
return networkUpgradeUnsupported(_that);case WalletErrorKind_ConsensusGraceExpired() when consensusGraceExpired != null:
return consensusGraceExpired(_that);case WalletErrorKind_ConsensusNotEvaluated() when consensusNotEvaluated != null:
return consensusNotEvaluated(_that);case WalletErrorKind_SyncRunning() when syncRunning != null:
return syncRunning(_that);case WalletErrorKind_InsufficientFunds() when insufficientFunds != null:
return insufficientFunds(_that);case WalletErrorKind_SendAmountRequired() when sendAmountRequired != null:
return sendAmountRequired(_that);case WalletErrorKind_ProposalAlreadyUsed() when proposalAlreadyUsed != null:
return proposalAlreadyUsed(_that);case WalletErrorKind_ProposalStale() when proposalStale != null:
return proposalStale(_that);case WalletErrorKind_ProposeFailed() when proposeFailed != null:
return proposeFailed(_that);case WalletErrorKind_ProposeTransient() when proposeTransient != null:
return proposeTransient(_that);case WalletErrorKind_SignFailed() when signFailed != null:
return signFailed(_that);case WalletErrorKind_QueuedSendStale() when queuedSendStale != null:
return queuedSendStale(_that);case WalletErrorKind_QueuedSendsFull() when queuedSendsFull != null:
return queuedSendsFull(_that);case WalletErrorKind_TexSendLimitReached() when texSendLimitReached != null:
return texSendLimitReached(_that);case WalletErrorKind_Io() when io != null:
return io(_that);case WalletErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult map<TResult extends Object?>({required TResult Function( WalletErrorKind_InvalidSeedLength value)  invalidSeedLength,required TResult Function( WalletErrorKind_InvalidMnemonic value)  invalidMnemonic,required TResult Function( WalletErrorKind_NoMnemonic value)  noMnemonic,required TResult Function( WalletErrorKind_SeedRequired value)  seedRequired,required TResult Function( WalletErrorKind_SeedMismatch value)  seedMismatch,required TResult Function( WalletErrorKind_KeyDerivation value)  keyDerivation,required TResult Function( WalletErrorKind_WatchOnly value)  watchOnly,required TResult Function( WalletErrorKind_InvalidViewingKey value)  invalidViewingKey,required TResult Function( WalletErrorKind_InvalidEndpoint value)  invalidEndpoint,required TResult Function( WalletErrorKind_BirthdayInFuture value)  birthdayInFuture,required TResult Function( WalletErrorKind_InvalidDbDir value)  invalidDbDir,required TResult Function( WalletErrorKind_InvalidEndpointAuth value)  invalidEndpointAuth,required TResult Function( WalletErrorKind_BroadcastJitterTooLong value)  broadcastJitterTooLong,required TResult Function( WalletErrorKind_AmountOutOfRange value)  amountOutOfRange,required TResult Function( WalletErrorKind_MemoTooLong value)  memoTooLong,required TResult Function( WalletErrorKind_ReservedMemoNotSendable value)  reservedMemoNotSendable,required TResult Function( WalletErrorKind_MemoRequiresShieldedRecipient value)  memoRequiresShieldedRecipient,required TResult Function( WalletErrorKind_AddressInvalid value)  addressInvalid,required TResult Function( WalletErrorKind_MemoInvalid value)  memoInvalid,required TResult Function( WalletErrorKind_MemoConflict value)  memoConflict,required TResult Function( WalletErrorKind_ZeroValuedTransparentOutput value)  zeroValuedTransparentOutput,required TResult Function( WalletErrorKind_PaymentUriInvalid value)  paymentUriInvalid,required TResult Function( WalletErrorKind_TxidInvalid value)  txidInvalid,required TResult Function( WalletErrorKind_MachineMemoScopeInvalid value)  machineMemoScopeInvalid,required TResult Function( WalletErrorKind_KeystoreUnavailable value)  keystoreUnavailable,required TResult Function( WalletErrorKind_KeystoreInconsistent value)  keystoreInconsistent,required TResult Function( WalletErrorKind_SealVersionUnsupported value)  sealVersionUnsupported,required TResult Function( WalletErrorKind_SealInvalid value)  sealInvalid,required TResult Function( WalletErrorKind_VaultAbsent value)  vaultAbsent,required TResult Function( WalletErrorKind_WrapArtifactInvalid value)  wrapArtifactInvalid,required TResult Function( WalletErrorKind_WrapVersionUnsupported value)  wrapVersionUnsupported,required TResult Function( WalletErrorKind_NotFound value)  notFound,required TResult Function( WalletErrorKind_NetworkMismatch value)  networkMismatch,required TResult Function( WalletErrorKind_ProvisioningIncomplete value)  provisioningIncomplete,required TResult Function( WalletErrorKind_WalletAlreadyExists value)  walletAlreadyExists,required TResult Function( WalletErrorKind_StoreCorrupt value)  storeCorrupt,required TResult Function( WalletErrorKind_DiskFull value)  diskFull,required TResult Function( WalletErrorKind_StoreBusy value)  storeBusy,required TResult Function( WalletErrorKind_WalletAlreadyOpen value)  walletAlreadyOpen,required TResult Function( WalletErrorKind_WalletBusy value)  walletBusy,required TResult Function( WalletErrorKind_InvalidState value)  invalidState,required TResult Function( WalletErrorKind_WalletOpen value)  walletOpen,required TResult Function( WalletErrorKind_SyncServerNotOffered value)  syncServerNotOffered,required TResult Function( WalletErrorKind_SyncServerUnreachable value)  syncServerUnreachable,required TResult Function( WalletErrorKind_WipeWithPendingSwap value)  wipeWithPendingSwap,required TResult Function( WalletErrorKind_RescanWithInFlightSend value)  rescanWithInFlightSend,required TResult Function( WalletErrorKind_SwapAddressCheckRefused value)  swapAddressCheckRefused,required TResult Function( WalletErrorKind_Sync value)  sync_,required TResult Function( WalletErrorKind_NetworkUpgradeUnsupported value)  networkUpgradeUnsupported,required TResult Function( WalletErrorKind_ConsensusGraceExpired value)  consensusGraceExpired,required TResult Function( WalletErrorKind_ConsensusNotEvaluated value)  consensusNotEvaluated,required TResult Function( WalletErrorKind_SyncRunning value)  syncRunning,required TResult Function( WalletErrorKind_InsufficientFunds value)  insufficientFunds,required TResult Function( WalletErrorKind_SendAmountRequired value)  sendAmountRequired,required TResult Function( WalletErrorKind_ProposalAlreadyUsed value)  proposalAlreadyUsed,required TResult Function( WalletErrorKind_ProposalStale value)  proposalStale,required TResult Function( WalletErrorKind_ProposeFailed value)  proposeFailed,required TResult Function( WalletErrorKind_ProposeTransient value)  proposeTransient,required TResult Function( WalletErrorKind_SignFailed value)  signFailed,required TResult Function( WalletErrorKind_QueuedSendStale value)  queuedSendStale,required TResult Function( WalletErrorKind_QueuedSendsFull value)  queuedSendsFull,required TResult Function( WalletErrorKind_TexSendLimitReached value)  texSendLimitReached,required TResult Function( WalletErrorKind_Io value)  io,required TResult Function( WalletErrorKind_Unknown value)  unknown,}){
final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength():
return invalidSeedLength(_that);case WalletErrorKind_InvalidMnemonic():
return invalidMnemonic(_that);case WalletErrorKind_NoMnemonic():
return noMnemonic(_that);case WalletErrorKind_SeedRequired():
return seedRequired(_that);case WalletErrorKind_SeedMismatch():
return seedMismatch(_that);case WalletErrorKind_KeyDerivation():
return keyDerivation(_that);case WalletErrorKind_WatchOnly():
return watchOnly(_that);case WalletErrorKind_InvalidViewingKey():
return invalidViewingKey(_that);case WalletErrorKind_InvalidEndpoint():
return invalidEndpoint(_that);case WalletErrorKind_BirthdayInFuture():
return birthdayInFuture(_that);case WalletErrorKind_InvalidDbDir():
return invalidDbDir(_that);case WalletErrorKind_InvalidEndpointAuth():
return invalidEndpointAuth(_that);case WalletErrorKind_BroadcastJitterTooLong():
return broadcastJitterTooLong(_that);case WalletErrorKind_AmountOutOfRange():
return amountOutOfRange(_that);case WalletErrorKind_MemoTooLong():
return memoTooLong(_that);case WalletErrorKind_ReservedMemoNotSendable():
return reservedMemoNotSendable(_that);case WalletErrorKind_MemoRequiresShieldedRecipient():
return memoRequiresShieldedRecipient(_that);case WalletErrorKind_AddressInvalid():
return addressInvalid(_that);case WalletErrorKind_MemoInvalid():
return memoInvalid(_that);case WalletErrorKind_MemoConflict():
return memoConflict(_that);case WalletErrorKind_ZeroValuedTransparentOutput():
return zeroValuedTransparentOutput(_that);case WalletErrorKind_PaymentUriInvalid():
return paymentUriInvalid(_that);case WalletErrorKind_TxidInvalid():
return txidInvalid(_that);case WalletErrorKind_MachineMemoScopeInvalid():
return machineMemoScopeInvalid(_that);case WalletErrorKind_KeystoreUnavailable():
return keystoreUnavailable(_that);case WalletErrorKind_KeystoreInconsistent():
return keystoreInconsistent(_that);case WalletErrorKind_SealVersionUnsupported():
return sealVersionUnsupported(_that);case WalletErrorKind_SealInvalid():
return sealInvalid(_that);case WalletErrorKind_VaultAbsent():
return vaultAbsent(_that);case WalletErrorKind_WrapArtifactInvalid():
return wrapArtifactInvalid(_that);case WalletErrorKind_WrapVersionUnsupported():
return wrapVersionUnsupported(_that);case WalletErrorKind_NotFound():
return notFound(_that);case WalletErrorKind_NetworkMismatch():
return networkMismatch(_that);case WalletErrorKind_ProvisioningIncomplete():
return provisioningIncomplete(_that);case WalletErrorKind_WalletAlreadyExists():
return walletAlreadyExists(_that);case WalletErrorKind_StoreCorrupt():
return storeCorrupt(_that);case WalletErrorKind_DiskFull():
return diskFull(_that);case WalletErrorKind_StoreBusy():
return storeBusy(_that);case WalletErrorKind_WalletAlreadyOpen():
return walletAlreadyOpen(_that);case WalletErrorKind_WalletBusy():
return walletBusy(_that);case WalletErrorKind_InvalidState():
return invalidState(_that);case WalletErrorKind_WalletOpen():
return walletOpen(_that);case WalletErrorKind_SyncServerNotOffered():
return syncServerNotOffered(_that);case WalletErrorKind_SyncServerUnreachable():
return syncServerUnreachable(_that);case WalletErrorKind_WipeWithPendingSwap():
return wipeWithPendingSwap(_that);case WalletErrorKind_RescanWithInFlightSend():
return rescanWithInFlightSend(_that);case WalletErrorKind_SwapAddressCheckRefused():
return swapAddressCheckRefused(_that);case WalletErrorKind_Sync():
return sync_(_that);case WalletErrorKind_NetworkUpgradeUnsupported():
return networkUpgradeUnsupported(_that);case WalletErrorKind_ConsensusGraceExpired():
return consensusGraceExpired(_that);case WalletErrorKind_ConsensusNotEvaluated():
return consensusNotEvaluated(_that);case WalletErrorKind_SyncRunning():
return syncRunning(_that);case WalletErrorKind_InsufficientFunds():
return insufficientFunds(_that);case WalletErrorKind_SendAmountRequired():
return sendAmountRequired(_that);case WalletErrorKind_ProposalAlreadyUsed():
return proposalAlreadyUsed(_that);case WalletErrorKind_ProposalStale():
return proposalStale(_that);case WalletErrorKind_ProposeFailed():
return proposeFailed(_that);case WalletErrorKind_ProposeTransient():
return proposeTransient(_that);case WalletErrorKind_SignFailed():
return signFailed(_that);case WalletErrorKind_QueuedSendStale():
return queuedSendStale(_that);case WalletErrorKind_QueuedSendsFull():
return queuedSendsFull(_that);case WalletErrorKind_TexSendLimitReached():
return texSendLimitReached(_that);case WalletErrorKind_Io():
return io(_that);case WalletErrorKind_Unknown():
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

@optionalTypeArgs TResult? mapOrNull<TResult extends Object?>({TResult? Function( WalletErrorKind_InvalidSeedLength value)?  invalidSeedLength,TResult? Function( WalletErrorKind_InvalidMnemonic value)?  invalidMnemonic,TResult? Function( WalletErrorKind_NoMnemonic value)?  noMnemonic,TResult? Function( WalletErrorKind_SeedRequired value)?  seedRequired,TResult? Function( WalletErrorKind_SeedMismatch value)?  seedMismatch,TResult? Function( WalletErrorKind_KeyDerivation value)?  keyDerivation,TResult? Function( WalletErrorKind_WatchOnly value)?  watchOnly,TResult? Function( WalletErrorKind_InvalidViewingKey value)?  invalidViewingKey,TResult? Function( WalletErrorKind_InvalidEndpoint value)?  invalidEndpoint,TResult? Function( WalletErrorKind_BirthdayInFuture value)?  birthdayInFuture,TResult? Function( WalletErrorKind_InvalidDbDir value)?  invalidDbDir,TResult? Function( WalletErrorKind_InvalidEndpointAuth value)?  invalidEndpointAuth,TResult? Function( WalletErrorKind_BroadcastJitterTooLong value)?  broadcastJitterTooLong,TResult? Function( WalletErrorKind_AmountOutOfRange value)?  amountOutOfRange,TResult? Function( WalletErrorKind_MemoTooLong value)?  memoTooLong,TResult? Function( WalletErrorKind_ReservedMemoNotSendable value)?  reservedMemoNotSendable,TResult? Function( WalletErrorKind_MemoRequiresShieldedRecipient value)?  memoRequiresShieldedRecipient,TResult? Function( WalletErrorKind_AddressInvalid value)?  addressInvalid,TResult? Function( WalletErrorKind_MemoInvalid value)?  memoInvalid,TResult? Function( WalletErrorKind_MemoConflict value)?  memoConflict,TResult? Function( WalletErrorKind_ZeroValuedTransparentOutput value)?  zeroValuedTransparentOutput,TResult? Function( WalletErrorKind_PaymentUriInvalid value)?  paymentUriInvalid,TResult? Function( WalletErrorKind_TxidInvalid value)?  txidInvalid,TResult? Function( WalletErrorKind_MachineMemoScopeInvalid value)?  machineMemoScopeInvalid,TResult? Function( WalletErrorKind_KeystoreUnavailable value)?  keystoreUnavailable,TResult? Function( WalletErrorKind_KeystoreInconsistent value)?  keystoreInconsistent,TResult? Function( WalletErrorKind_SealVersionUnsupported value)?  sealVersionUnsupported,TResult? Function( WalletErrorKind_SealInvalid value)?  sealInvalid,TResult? Function( WalletErrorKind_VaultAbsent value)?  vaultAbsent,TResult? Function( WalletErrorKind_WrapArtifactInvalid value)?  wrapArtifactInvalid,TResult? Function( WalletErrorKind_WrapVersionUnsupported value)?  wrapVersionUnsupported,TResult? Function( WalletErrorKind_NotFound value)?  notFound,TResult? Function( WalletErrorKind_NetworkMismatch value)?  networkMismatch,TResult? Function( WalletErrorKind_ProvisioningIncomplete value)?  provisioningIncomplete,TResult? Function( WalletErrorKind_WalletAlreadyExists value)?  walletAlreadyExists,TResult? Function( WalletErrorKind_StoreCorrupt value)?  storeCorrupt,TResult? Function( WalletErrorKind_DiskFull value)?  diskFull,TResult? Function( WalletErrorKind_StoreBusy value)?  storeBusy,TResult? Function( WalletErrorKind_WalletAlreadyOpen value)?  walletAlreadyOpen,TResult? Function( WalletErrorKind_WalletBusy value)?  walletBusy,TResult? Function( WalletErrorKind_InvalidState value)?  invalidState,TResult? Function( WalletErrorKind_WalletOpen value)?  walletOpen,TResult? Function( WalletErrorKind_SyncServerNotOffered value)?  syncServerNotOffered,TResult? Function( WalletErrorKind_SyncServerUnreachable value)?  syncServerUnreachable,TResult? Function( WalletErrorKind_WipeWithPendingSwap value)?  wipeWithPendingSwap,TResult? Function( WalletErrorKind_RescanWithInFlightSend value)?  rescanWithInFlightSend,TResult? Function( WalletErrorKind_SwapAddressCheckRefused value)?  swapAddressCheckRefused,TResult? Function( WalletErrorKind_Sync value)?  sync_,TResult? Function( WalletErrorKind_NetworkUpgradeUnsupported value)?  networkUpgradeUnsupported,TResult? Function( WalletErrorKind_ConsensusGraceExpired value)?  consensusGraceExpired,TResult? Function( WalletErrorKind_ConsensusNotEvaluated value)?  consensusNotEvaluated,TResult? Function( WalletErrorKind_SyncRunning value)?  syncRunning,TResult? Function( WalletErrorKind_InsufficientFunds value)?  insufficientFunds,TResult? Function( WalletErrorKind_SendAmountRequired value)?  sendAmountRequired,TResult? Function( WalletErrorKind_ProposalAlreadyUsed value)?  proposalAlreadyUsed,TResult? Function( WalletErrorKind_ProposalStale value)?  proposalStale,TResult? Function( WalletErrorKind_ProposeFailed value)?  proposeFailed,TResult? Function( WalletErrorKind_ProposeTransient value)?  proposeTransient,TResult? Function( WalletErrorKind_SignFailed value)?  signFailed,TResult? Function( WalletErrorKind_QueuedSendStale value)?  queuedSendStale,TResult? Function( WalletErrorKind_QueuedSendsFull value)?  queuedSendsFull,TResult? Function( WalletErrorKind_TexSendLimitReached value)?  texSendLimitReached,TResult? Function( WalletErrorKind_Io value)?  io,TResult? Function( WalletErrorKind_Unknown value)?  unknown,}){
final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength() when invalidSeedLength != null:
return invalidSeedLength(_that);case WalletErrorKind_InvalidMnemonic() when invalidMnemonic != null:
return invalidMnemonic(_that);case WalletErrorKind_NoMnemonic() when noMnemonic != null:
return noMnemonic(_that);case WalletErrorKind_SeedRequired() when seedRequired != null:
return seedRequired(_that);case WalletErrorKind_SeedMismatch() when seedMismatch != null:
return seedMismatch(_that);case WalletErrorKind_KeyDerivation() when keyDerivation != null:
return keyDerivation(_that);case WalletErrorKind_WatchOnly() when watchOnly != null:
return watchOnly(_that);case WalletErrorKind_InvalidViewingKey() when invalidViewingKey != null:
return invalidViewingKey(_that);case WalletErrorKind_InvalidEndpoint() when invalidEndpoint != null:
return invalidEndpoint(_that);case WalletErrorKind_BirthdayInFuture() when birthdayInFuture != null:
return birthdayInFuture(_that);case WalletErrorKind_InvalidDbDir() when invalidDbDir != null:
return invalidDbDir(_that);case WalletErrorKind_InvalidEndpointAuth() when invalidEndpointAuth != null:
return invalidEndpointAuth(_that);case WalletErrorKind_BroadcastJitterTooLong() when broadcastJitterTooLong != null:
return broadcastJitterTooLong(_that);case WalletErrorKind_AmountOutOfRange() when amountOutOfRange != null:
return amountOutOfRange(_that);case WalletErrorKind_MemoTooLong() when memoTooLong != null:
return memoTooLong(_that);case WalletErrorKind_ReservedMemoNotSendable() when reservedMemoNotSendable != null:
return reservedMemoNotSendable(_that);case WalletErrorKind_MemoRequiresShieldedRecipient() when memoRequiresShieldedRecipient != null:
return memoRequiresShieldedRecipient(_that);case WalletErrorKind_AddressInvalid() when addressInvalid != null:
return addressInvalid(_that);case WalletErrorKind_MemoInvalid() when memoInvalid != null:
return memoInvalid(_that);case WalletErrorKind_MemoConflict() when memoConflict != null:
return memoConflict(_that);case WalletErrorKind_ZeroValuedTransparentOutput() when zeroValuedTransparentOutput != null:
return zeroValuedTransparentOutput(_that);case WalletErrorKind_PaymentUriInvalid() when paymentUriInvalid != null:
return paymentUriInvalid(_that);case WalletErrorKind_TxidInvalid() when txidInvalid != null:
return txidInvalid(_that);case WalletErrorKind_MachineMemoScopeInvalid() when machineMemoScopeInvalid != null:
return machineMemoScopeInvalid(_that);case WalletErrorKind_KeystoreUnavailable() when keystoreUnavailable != null:
return keystoreUnavailable(_that);case WalletErrorKind_KeystoreInconsistent() when keystoreInconsistent != null:
return keystoreInconsistent(_that);case WalletErrorKind_SealVersionUnsupported() when sealVersionUnsupported != null:
return sealVersionUnsupported(_that);case WalletErrorKind_SealInvalid() when sealInvalid != null:
return sealInvalid(_that);case WalletErrorKind_VaultAbsent() when vaultAbsent != null:
return vaultAbsent(_that);case WalletErrorKind_WrapArtifactInvalid() when wrapArtifactInvalid != null:
return wrapArtifactInvalid(_that);case WalletErrorKind_WrapVersionUnsupported() when wrapVersionUnsupported != null:
return wrapVersionUnsupported(_that);case WalletErrorKind_NotFound() when notFound != null:
return notFound(_that);case WalletErrorKind_NetworkMismatch() when networkMismatch != null:
return networkMismatch(_that);case WalletErrorKind_ProvisioningIncomplete() when provisioningIncomplete != null:
return provisioningIncomplete(_that);case WalletErrorKind_WalletAlreadyExists() when walletAlreadyExists != null:
return walletAlreadyExists(_that);case WalletErrorKind_StoreCorrupt() when storeCorrupt != null:
return storeCorrupt(_that);case WalletErrorKind_DiskFull() when diskFull != null:
return diskFull(_that);case WalletErrorKind_StoreBusy() when storeBusy != null:
return storeBusy(_that);case WalletErrorKind_WalletAlreadyOpen() when walletAlreadyOpen != null:
return walletAlreadyOpen(_that);case WalletErrorKind_WalletBusy() when walletBusy != null:
return walletBusy(_that);case WalletErrorKind_InvalidState() when invalidState != null:
return invalidState(_that);case WalletErrorKind_WalletOpen() when walletOpen != null:
return walletOpen(_that);case WalletErrorKind_SyncServerNotOffered() when syncServerNotOffered != null:
return syncServerNotOffered(_that);case WalletErrorKind_SyncServerUnreachable() when syncServerUnreachable != null:
return syncServerUnreachable(_that);case WalletErrorKind_WipeWithPendingSwap() when wipeWithPendingSwap != null:
return wipeWithPendingSwap(_that);case WalletErrorKind_RescanWithInFlightSend() when rescanWithInFlightSend != null:
return rescanWithInFlightSend(_that);case WalletErrorKind_SwapAddressCheckRefused() when swapAddressCheckRefused != null:
return swapAddressCheckRefused(_that);case WalletErrorKind_Sync() when sync_ != null:
return sync_(_that);case WalletErrorKind_NetworkUpgradeUnsupported() when networkUpgradeUnsupported != null:
return networkUpgradeUnsupported(_that);case WalletErrorKind_ConsensusGraceExpired() when consensusGraceExpired != null:
return consensusGraceExpired(_that);case WalletErrorKind_ConsensusNotEvaluated() when consensusNotEvaluated != null:
return consensusNotEvaluated(_that);case WalletErrorKind_SyncRunning() when syncRunning != null:
return syncRunning(_that);case WalletErrorKind_InsufficientFunds() when insufficientFunds != null:
return insufficientFunds(_that);case WalletErrorKind_SendAmountRequired() when sendAmountRequired != null:
return sendAmountRequired(_that);case WalletErrorKind_ProposalAlreadyUsed() when proposalAlreadyUsed != null:
return proposalAlreadyUsed(_that);case WalletErrorKind_ProposalStale() when proposalStale != null:
return proposalStale(_that);case WalletErrorKind_ProposeFailed() when proposeFailed != null:
return proposeFailed(_that);case WalletErrorKind_ProposeTransient() when proposeTransient != null:
return proposeTransient(_that);case WalletErrorKind_SignFailed() when signFailed != null:
return signFailed(_that);case WalletErrorKind_QueuedSendStale() when queuedSendStale != null:
return queuedSendStale(_that);case WalletErrorKind_QueuedSendsFull() when queuedSendsFull != null:
return queuedSendsFull(_that);case WalletErrorKind_TexSendLimitReached() when texSendLimitReached != null:
return texSendLimitReached(_that);case WalletErrorKind_Io() when io != null:
return io(_that);case WalletErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult maybeWhen<TResult extends Object?>({TResult Function( BigInt len)?  invalidSeedLength,TResult Function( int? wordIndex)?  invalidMnemonic,TResult Function()?  noMnemonic,TResult Function()?  seedRequired,TResult Function()?  seedMismatch,TResult Function()?  keyDerivation,TResult Function()?  watchOnly,TResult Function()?  invalidViewingKey,TResult Function( String reason)?  invalidEndpoint,TResult Function()?  birthdayInFuture,TResult Function( String reason)?  invalidDbDir,TResult Function( String reason)?  invalidEndpointAuth,TResult Function( BigInt maxMs,  BigInt ceilingMs)?  broadcastJitterTooLong,TResult Function()?  amountOutOfRange,TResult Function( BigInt len,  BigInt max)?  memoTooLong,TResult Function()?  reservedMemoNotSendable,TResult Function()?  memoRequiresShieldedRecipient,TResult Function()?  addressInvalid,TResult Function()?  memoInvalid,TResult Function()?  memoConflict,TResult Function()?  zeroValuedTransparentOutput,TResult Function()?  paymentUriInvalid,TResult Function()?  txidInvalid,TResult Function( String reason)?  machineMemoScopeInvalid,TResult Function()?  keystoreUnavailable,TResult Function( bool permanentlyInvalidated)?  keystoreInconsistent,TResult Function( int found)?  sealVersionUnsupported,TResult Function()?  sealInvalid,TResult Function()?  vaultAbsent,TResult Function()?  wrapArtifactInvalid,TResult Function( int found)?  wrapVersionUnsupported,TResult Function()?  notFound,TResult Function()?  networkMismatch,TResult Function()?  provisioningIncomplete,TResult Function()?  walletAlreadyExists,TResult Function()?  storeCorrupt,TResult Function()?  diskFull,TResult Function()?  storeBusy,TResult Function()?  walletAlreadyOpen,TResult Function( LifecyclePhase phase)?  walletBusy,TResult Function( LifecyclePhase phase)?  invalidState,TResult Function()?  walletOpen,TResult Function()?  syncServerNotOffered,TResult Function()?  syncServerUnreachable,TResult Function( int count)?  wipeWithPendingSwap,TResult Function()?  rescanWithInFlightSend,TResult Function( SwapAddressCheckRefusal reason)?  swapAddressCheckRefused,TResult Function( StallReason stall)?  sync_,TResult Function( int expectedBranchId,  int? endpointBranchId,  int judgedAtHeight)?  networkUpgradeUnsupported,TResult Function( GraceExpiry by,  int? blocksSinceLastCurrent)?  consensusGraceExpired,TResult Function()?  consensusNotEvaluated,TResult Function()?  syncRunning,TResult Function( PlatformInt64 availableZat,  PlatformInt64 requiredZat,  PlatformInt64 pendingIncomingZat)?  insufficientFunds,TResult Function()?  sendAmountRequired,TResult Function()?  proposalAlreadyUsed,TResult Function()?  proposalStale,TResult Function()?  proposeFailed,TResult Function()?  proposeTransient,TResult Function()?  signFailed,TResult Function()?  queuedSendStale,TResult Function()?  queuedSendsFull,TResult Function()?  texSendLimitReached,TResult Function()?  io,TResult Function()?  unknown,required TResult orElse(),}) {final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength() when invalidSeedLength != null:
return invalidSeedLength(_that.len);case WalletErrorKind_InvalidMnemonic() when invalidMnemonic != null:
return invalidMnemonic(_that.wordIndex);case WalletErrorKind_NoMnemonic() when noMnemonic != null:
return noMnemonic();case WalletErrorKind_SeedRequired() when seedRequired != null:
return seedRequired();case WalletErrorKind_SeedMismatch() when seedMismatch != null:
return seedMismatch();case WalletErrorKind_KeyDerivation() when keyDerivation != null:
return keyDerivation();case WalletErrorKind_WatchOnly() when watchOnly != null:
return watchOnly();case WalletErrorKind_InvalidViewingKey() when invalidViewingKey != null:
return invalidViewingKey();case WalletErrorKind_InvalidEndpoint() when invalidEndpoint != null:
return invalidEndpoint(_that.reason);case WalletErrorKind_BirthdayInFuture() when birthdayInFuture != null:
return birthdayInFuture();case WalletErrorKind_InvalidDbDir() when invalidDbDir != null:
return invalidDbDir(_that.reason);case WalletErrorKind_InvalidEndpointAuth() when invalidEndpointAuth != null:
return invalidEndpointAuth(_that.reason);case WalletErrorKind_BroadcastJitterTooLong() when broadcastJitterTooLong != null:
return broadcastJitterTooLong(_that.maxMs,_that.ceilingMs);case WalletErrorKind_AmountOutOfRange() when amountOutOfRange != null:
return amountOutOfRange();case WalletErrorKind_MemoTooLong() when memoTooLong != null:
return memoTooLong(_that.len,_that.max);case WalletErrorKind_ReservedMemoNotSendable() when reservedMemoNotSendable != null:
return reservedMemoNotSendable();case WalletErrorKind_MemoRequiresShieldedRecipient() when memoRequiresShieldedRecipient != null:
return memoRequiresShieldedRecipient();case WalletErrorKind_AddressInvalid() when addressInvalid != null:
return addressInvalid();case WalletErrorKind_MemoInvalid() when memoInvalid != null:
return memoInvalid();case WalletErrorKind_MemoConflict() when memoConflict != null:
return memoConflict();case WalletErrorKind_ZeroValuedTransparentOutput() when zeroValuedTransparentOutput != null:
return zeroValuedTransparentOutput();case WalletErrorKind_PaymentUriInvalid() when paymentUriInvalid != null:
return paymentUriInvalid();case WalletErrorKind_TxidInvalid() when txidInvalid != null:
return txidInvalid();case WalletErrorKind_MachineMemoScopeInvalid() when machineMemoScopeInvalid != null:
return machineMemoScopeInvalid(_that.reason);case WalletErrorKind_KeystoreUnavailable() when keystoreUnavailable != null:
return keystoreUnavailable();case WalletErrorKind_KeystoreInconsistent() when keystoreInconsistent != null:
return keystoreInconsistent(_that.permanentlyInvalidated);case WalletErrorKind_SealVersionUnsupported() when sealVersionUnsupported != null:
return sealVersionUnsupported(_that.found);case WalletErrorKind_SealInvalid() when sealInvalid != null:
return sealInvalid();case WalletErrorKind_VaultAbsent() when vaultAbsent != null:
return vaultAbsent();case WalletErrorKind_WrapArtifactInvalid() when wrapArtifactInvalid != null:
return wrapArtifactInvalid();case WalletErrorKind_WrapVersionUnsupported() when wrapVersionUnsupported != null:
return wrapVersionUnsupported(_that.found);case WalletErrorKind_NotFound() when notFound != null:
return notFound();case WalletErrorKind_NetworkMismatch() when networkMismatch != null:
return networkMismatch();case WalletErrorKind_ProvisioningIncomplete() when provisioningIncomplete != null:
return provisioningIncomplete();case WalletErrorKind_WalletAlreadyExists() when walletAlreadyExists != null:
return walletAlreadyExists();case WalletErrorKind_StoreCorrupt() when storeCorrupt != null:
return storeCorrupt();case WalletErrorKind_DiskFull() when diskFull != null:
return diskFull();case WalletErrorKind_StoreBusy() when storeBusy != null:
return storeBusy();case WalletErrorKind_WalletAlreadyOpen() when walletAlreadyOpen != null:
return walletAlreadyOpen();case WalletErrorKind_WalletBusy() when walletBusy != null:
return walletBusy(_that.phase);case WalletErrorKind_InvalidState() when invalidState != null:
return invalidState(_that.phase);case WalletErrorKind_WalletOpen() when walletOpen != null:
return walletOpen();case WalletErrorKind_SyncServerNotOffered() when syncServerNotOffered != null:
return syncServerNotOffered();case WalletErrorKind_SyncServerUnreachable() when syncServerUnreachable != null:
return syncServerUnreachable();case WalletErrorKind_WipeWithPendingSwap() when wipeWithPendingSwap != null:
return wipeWithPendingSwap(_that.count);case WalletErrorKind_RescanWithInFlightSend() when rescanWithInFlightSend != null:
return rescanWithInFlightSend();case WalletErrorKind_SwapAddressCheckRefused() when swapAddressCheckRefused != null:
return swapAddressCheckRefused(_that.reason);case WalletErrorKind_Sync() when sync_ != null:
return sync_(_that.stall);case WalletErrorKind_NetworkUpgradeUnsupported() when networkUpgradeUnsupported != null:
return networkUpgradeUnsupported(_that.expectedBranchId,_that.endpointBranchId,_that.judgedAtHeight);case WalletErrorKind_ConsensusGraceExpired() when consensusGraceExpired != null:
return consensusGraceExpired(_that.by,_that.blocksSinceLastCurrent);case WalletErrorKind_ConsensusNotEvaluated() when consensusNotEvaluated != null:
return consensusNotEvaluated();case WalletErrorKind_SyncRunning() when syncRunning != null:
return syncRunning();case WalletErrorKind_InsufficientFunds() when insufficientFunds != null:
return insufficientFunds(_that.availableZat,_that.requiredZat,_that.pendingIncomingZat);case WalletErrorKind_SendAmountRequired() when sendAmountRequired != null:
return sendAmountRequired();case WalletErrorKind_ProposalAlreadyUsed() when proposalAlreadyUsed != null:
return proposalAlreadyUsed();case WalletErrorKind_ProposalStale() when proposalStale != null:
return proposalStale();case WalletErrorKind_ProposeFailed() when proposeFailed != null:
return proposeFailed();case WalletErrorKind_ProposeTransient() when proposeTransient != null:
return proposeTransient();case WalletErrorKind_SignFailed() when signFailed != null:
return signFailed();case WalletErrorKind_QueuedSendStale() when queuedSendStale != null:
return queuedSendStale();case WalletErrorKind_QueuedSendsFull() when queuedSendsFull != null:
return queuedSendsFull();case WalletErrorKind_TexSendLimitReached() when texSendLimitReached != null:
return texSendLimitReached();case WalletErrorKind_Io() when io != null:
return io();case WalletErrorKind_Unknown() when unknown != null:
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

@optionalTypeArgs TResult when<TResult extends Object?>({required TResult Function( BigInt len)  invalidSeedLength,required TResult Function( int? wordIndex)  invalidMnemonic,required TResult Function()  noMnemonic,required TResult Function()  seedRequired,required TResult Function()  seedMismatch,required TResult Function()  keyDerivation,required TResult Function()  watchOnly,required TResult Function()  invalidViewingKey,required TResult Function( String reason)  invalidEndpoint,required TResult Function()  birthdayInFuture,required TResult Function( String reason)  invalidDbDir,required TResult Function( String reason)  invalidEndpointAuth,required TResult Function( BigInt maxMs,  BigInt ceilingMs)  broadcastJitterTooLong,required TResult Function()  amountOutOfRange,required TResult Function( BigInt len,  BigInt max)  memoTooLong,required TResult Function()  reservedMemoNotSendable,required TResult Function()  memoRequiresShieldedRecipient,required TResult Function()  addressInvalid,required TResult Function()  memoInvalid,required TResult Function()  memoConflict,required TResult Function()  zeroValuedTransparentOutput,required TResult Function()  paymentUriInvalid,required TResult Function()  txidInvalid,required TResult Function( String reason)  machineMemoScopeInvalid,required TResult Function()  keystoreUnavailable,required TResult Function( bool permanentlyInvalidated)  keystoreInconsistent,required TResult Function( int found)  sealVersionUnsupported,required TResult Function()  sealInvalid,required TResult Function()  vaultAbsent,required TResult Function()  wrapArtifactInvalid,required TResult Function( int found)  wrapVersionUnsupported,required TResult Function()  notFound,required TResult Function()  networkMismatch,required TResult Function()  provisioningIncomplete,required TResult Function()  walletAlreadyExists,required TResult Function()  storeCorrupt,required TResult Function()  diskFull,required TResult Function()  storeBusy,required TResult Function()  walletAlreadyOpen,required TResult Function( LifecyclePhase phase)  walletBusy,required TResult Function( LifecyclePhase phase)  invalidState,required TResult Function()  walletOpen,required TResult Function()  syncServerNotOffered,required TResult Function()  syncServerUnreachable,required TResult Function( int count)  wipeWithPendingSwap,required TResult Function()  rescanWithInFlightSend,required TResult Function( SwapAddressCheckRefusal reason)  swapAddressCheckRefused,required TResult Function( StallReason stall)  sync_,required TResult Function( int expectedBranchId,  int? endpointBranchId,  int judgedAtHeight)  networkUpgradeUnsupported,required TResult Function( GraceExpiry by,  int? blocksSinceLastCurrent)  consensusGraceExpired,required TResult Function()  consensusNotEvaluated,required TResult Function()  syncRunning,required TResult Function( PlatformInt64 availableZat,  PlatformInt64 requiredZat,  PlatformInt64 pendingIncomingZat)  insufficientFunds,required TResult Function()  sendAmountRequired,required TResult Function()  proposalAlreadyUsed,required TResult Function()  proposalStale,required TResult Function()  proposeFailed,required TResult Function()  proposeTransient,required TResult Function()  signFailed,required TResult Function()  queuedSendStale,required TResult Function()  queuedSendsFull,required TResult Function()  texSendLimitReached,required TResult Function()  io,required TResult Function()  unknown,}) {final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength():
return invalidSeedLength(_that.len);case WalletErrorKind_InvalidMnemonic():
return invalidMnemonic(_that.wordIndex);case WalletErrorKind_NoMnemonic():
return noMnemonic();case WalletErrorKind_SeedRequired():
return seedRequired();case WalletErrorKind_SeedMismatch():
return seedMismatch();case WalletErrorKind_KeyDerivation():
return keyDerivation();case WalletErrorKind_WatchOnly():
return watchOnly();case WalletErrorKind_InvalidViewingKey():
return invalidViewingKey();case WalletErrorKind_InvalidEndpoint():
return invalidEndpoint(_that.reason);case WalletErrorKind_BirthdayInFuture():
return birthdayInFuture();case WalletErrorKind_InvalidDbDir():
return invalidDbDir(_that.reason);case WalletErrorKind_InvalidEndpointAuth():
return invalidEndpointAuth(_that.reason);case WalletErrorKind_BroadcastJitterTooLong():
return broadcastJitterTooLong(_that.maxMs,_that.ceilingMs);case WalletErrorKind_AmountOutOfRange():
return amountOutOfRange();case WalletErrorKind_MemoTooLong():
return memoTooLong(_that.len,_that.max);case WalletErrorKind_ReservedMemoNotSendable():
return reservedMemoNotSendable();case WalletErrorKind_MemoRequiresShieldedRecipient():
return memoRequiresShieldedRecipient();case WalletErrorKind_AddressInvalid():
return addressInvalid();case WalletErrorKind_MemoInvalid():
return memoInvalid();case WalletErrorKind_MemoConflict():
return memoConflict();case WalletErrorKind_ZeroValuedTransparentOutput():
return zeroValuedTransparentOutput();case WalletErrorKind_PaymentUriInvalid():
return paymentUriInvalid();case WalletErrorKind_TxidInvalid():
return txidInvalid();case WalletErrorKind_MachineMemoScopeInvalid():
return machineMemoScopeInvalid(_that.reason);case WalletErrorKind_KeystoreUnavailable():
return keystoreUnavailable();case WalletErrorKind_KeystoreInconsistent():
return keystoreInconsistent(_that.permanentlyInvalidated);case WalletErrorKind_SealVersionUnsupported():
return sealVersionUnsupported(_that.found);case WalletErrorKind_SealInvalid():
return sealInvalid();case WalletErrorKind_VaultAbsent():
return vaultAbsent();case WalletErrorKind_WrapArtifactInvalid():
return wrapArtifactInvalid();case WalletErrorKind_WrapVersionUnsupported():
return wrapVersionUnsupported(_that.found);case WalletErrorKind_NotFound():
return notFound();case WalletErrorKind_NetworkMismatch():
return networkMismatch();case WalletErrorKind_ProvisioningIncomplete():
return provisioningIncomplete();case WalletErrorKind_WalletAlreadyExists():
return walletAlreadyExists();case WalletErrorKind_StoreCorrupt():
return storeCorrupt();case WalletErrorKind_DiskFull():
return diskFull();case WalletErrorKind_StoreBusy():
return storeBusy();case WalletErrorKind_WalletAlreadyOpen():
return walletAlreadyOpen();case WalletErrorKind_WalletBusy():
return walletBusy(_that.phase);case WalletErrorKind_InvalidState():
return invalidState(_that.phase);case WalletErrorKind_WalletOpen():
return walletOpen();case WalletErrorKind_SyncServerNotOffered():
return syncServerNotOffered();case WalletErrorKind_SyncServerUnreachable():
return syncServerUnreachable();case WalletErrorKind_WipeWithPendingSwap():
return wipeWithPendingSwap(_that.count);case WalletErrorKind_RescanWithInFlightSend():
return rescanWithInFlightSend();case WalletErrorKind_SwapAddressCheckRefused():
return swapAddressCheckRefused(_that.reason);case WalletErrorKind_Sync():
return sync_(_that.stall);case WalletErrorKind_NetworkUpgradeUnsupported():
return networkUpgradeUnsupported(_that.expectedBranchId,_that.endpointBranchId,_that.judgedAtHeight);case WalletErrorKind_ConsensusGraceExpired():
return consensusGraceExpired(_that.by,_that.blocksSinceLastCurrent);case WalletErrorKind_ConsensusNotEvaluated():
return consensusNotEvaluated();case WalletErrorKind_SyncRunning():
return syncRunning();case WalletErrorKind_InsufficientFunds():
return insufficientFunds(_that.availableZat,_that.requiredZat,_that.pendingIncomingZat);case WalletErrorKind_SendAmountRequired():
return sendAmountRequired();case WalletErrorKind_ProposalAlreadyUsed():
return proposalAlreadyUsed();case WalletErrorKind_ProposalStale():
return proposalStale();case WalletErrorKind_ProposeFailed():
return proposeFailed();case WalletErrorKind_ProposeTransient():
return proposeTransient();case WalletErrorKind_SignFailed():
return signFailed();case WalletErrorKind_QueuedSendStale():
return queuedSendStale();case WalletErrorKind_QueuedSendsFull():
return queuedSendsFull();case WalletErrorKind_TexSendLimitReached():
return texSendLimitReached();case WalletErrorKind_Io():
return io();case WalletErrorKind_Unknown():
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

@optionalTypeArgs TResult? whenOrNull<TResult extends Object?>({TResult? Function( BigInt len)?  invalidSeedLength,TResult? Function( int? wordIndex)?  invalidMnemonic,TResult? Function()?  noMnemonic,TResult? Function()?  seedRequired,TResult? Function()?  seedMismatch,TResult? Function()?  keyDerivation,TResult? Function()?  watchOnly,TResult? Function()?  invalidViewingKey,TResult? Function( String reason)?  invalidEndpoint,TResult? Function()?  birthdayInFuture,TResult? Function( String reason)?  invalidDbDir,TResult? Function( String reason)?  invalidEndpointAuth,TResult? Function( BigInt maxMs,  BigInt ceilingMs)?  broadcastJitterTooLong,TResult? Function()?  amountOutOfRange,TResult? Function( BigInt len,  BigInt max)?  memoTooLong,TResult? Function()?  reservedMemoNotSendable,TResult? Function()?  memoRequiresShieldedRecipient,TResult? Function()?  addressInvalid,TResult? Function()?  memoInvalid,TResult? Function()?  memoConflict,TResult? Function()?  zeroValuedTransparentOutput,TResult? Function()?  paymentUriInvalid,TResult? Function()?  txidInvalid,TResult? Function( String reason)?  machineMemoScopeInvalid,TResult? Function()?  keystoreUnavailable,TResult? Function( bool permanentlyInvalidated)?  keystoreInconsistent,TResult? Function( int found)?  sealVersionUnsupported,TResult? Function()?  sealInvalid,TResult? Function()?  vaultAbsent,TResult? Function()?  wrapArtifactInvalid,TResult? Function( int found)?  wrapVersionUnsupported,TResult? Function()?  notFound,TResult? Function()?  networkMismatch,TResult? Function()?  provisioningIncomplete,TResult? Function()?  walletAlreadyExists,TResult? Function()?  storeCorrupt,TResult? Function()?  diskFull,TResult? Function()?  storeBusy,TResult? Function()?  walletAlreadyOpen,TResult? Function( LifecyclePhase phase)?  walletBusy,TResult? Function( LifecyclePhase phase)?  invalidState,TResult? Function()?  walletOpen,TResult? Function()?  syncServerNotOffered,TResult? Function()?  syncServerUnreachable,TResult? Function( int count)?  wipeWithPendingSwap,TResult? Function()?  rescanWithInFlightSend,TResult? Function( SwapAddressCheckRefusal reason)?  swapAddressCheckRefused,TResult? Function( StallReason stall)?  sync_,TResult? Function( int expectedBranchId,  int? endpointBranchId,  int judgedAtHeight)?  networkUpgradeUnsupported,TResult? Function( GraceExpiry by,  int? blocksSinceLastCurrent)?  consensusGraceExpired,TResult? Function()?  consensusNotEvaluated,TResult? Function()?  syncRunning,TResult? Function( PlatformInt64 availableZat,  PlatformInt64 requiredZat,  PlatformInt64 pendingIncomingZat)?  insufficientFunds,TResult? Function()?  sendAmountRequired,TResult? Function()?  proposalAlreadyUsed,TResult? Function()?  proposalStale,TResult? Function()?  proposeFailed,TResult? Function()?  proposeTransient,TResult? Function()?  signFailed,TResult? Function()?  queuedSendStale,TResult? Function()?  queuedSendsFull,TResult? Function()?  texSendLimitReached,TResult? Function()?  io,TResult? Function()?  unknown,}) {final _that = this;
switch (_that) {
case WalletErrorKind_InvalidSeedLength() when invalidSeedLength != null:
return invalidSeedLength(_that.len);case WalletErrorKind_InvalidMnemonic() when invalidMnemonic != null:
return invalidMnemonic(_that.wordIndex);case WalletErrorKind_NoMnemonic() when noMnemonic != null:
return noMnemonic();case WalletErrorKind_SeedRequired() when seedRequired != null:
return seedRequired();case WalletErrorKind_SeedMismatch() when seedMismatch != null:
return seedMismatch();case WalletErrorKind_KeyDerivation() when keyDerivation != null:
return keyDerivation();case WalletErrorKind_WatchOnly() when watchOnly != null:
return watchOnly();case WalletErrorKind_InvalidViewingKey() when invalidViewingKey != null:
return invalidViewingKey();case WalletErrorKind_InvalidEndpoint() when invalidEndpoint != null:
return invalidEndpoint(_that.reason);case WalletErrorKind_BirthdayInFuture() when birthdayInFuture != null:
return birthdayInFuture();case WalletErrorKind_InvalidDbDir() when invalidDbDir != null:
return invalidDbDir(_that.reason);case WalletErrorKind_InvalidEndpointAuth() when invalidEndpointAuth != null:
return invalidEndpointAuth(_that.reason);case WalletErrorKind_BroadcastJitterTooLong() when broadcastJitterTooLong != null:
return broadcastJitterTooLong(_that.maxMs,_that.ceilingMs);case WalletErrorKind_AmountOutOfRange() when amountOutOfRange != null:
return amountOutOfRange();case WalletErrorKind_MemoTooLong() when memoTooLong != null:
return memoTooLong(_that.len,_that.max);case WalletErrorKind_ReservedMemoNotSendable() when reservedMemoNotSendable != null:
return reservedMemoNotSendable();case WalletErrorKind_MemoRequiresShieldedRecipient() when memoRequiresShieldedRecipient != null:
return memoRequiresShieldedRecipient();case WalletErrorKind_AddressInvalid() when addressInvalid != null:
return addressInvalid();case WalletErrorKind_MemoInvalid() when memoInvalid != null:
return memoInvalid();case WalletErrorKind_MemoConflict() when memoConflict != null:
return memoConflict();case WalletErrorKind_ZeroValuedTransparentOutput() when zeroValuedTransparentOutput != null:
return zeroValuedTransparentOutput();case WalletErrorKind_PaymentUriInvalid() when paymentUriInvalid != null:
return paymentUriInvalid();case WalletErrorKind_TxidInvalid() when txidInvalid != null:
return txidInvalid();case WalletErrorKind_MachineMemoScopeInvalid() when machineMemoScopeInvalid != null:
return machineMemoScopeInvalid(_that.reason);case WalletErrorKind_KeystoreUnavailable() when keystoreUnavailable != null:
return keystoreUnavailable();case WalletErrorKind_KeystoreInconsistent() when keystoreInconsistent != null:
return keystoreInconsistent(_that.permanentlyInvalidated);case WalletErrorKind_SealVersionUnsupported() when sealVersionUnsupported != null:
return sealVersionUnsupported(_that.found);case WalletErrorKind_SealInvalid() when sealInvalid != null:
return sealInvalid();case WalletErrorKind_VaultAbsent() when vaultAbsent != null:
return vaultAbsent();case WalletErrorKind_WrapArtifactInvalid() when wrapArtifactInvalid != null:
return wrapArtifactInvalid();case WalletErrorKind_WrapVersionUnsupported() when wrapVersionUnsupported != null:
return wrapVersionUnsupported(_that.found);case WalletErrorKind_NotFound() when notFound != null:
return notFound();case WalletErrorKind_NetworkMismatch() when networkMismatch != null:
return networkMismatch();case WalletErrorKind_ProvisioningIncomplete() when provisioningIncomplete != null:
return provisioningIncomplete();case WalletErrorKind_WalletAlreadyExists() when walletAlreadyExists != null:
return walletAlreadyExists();case WalletErrorKind_StoreCorrupt() when storeCorrupt != null:
return storeCorrupt();case WalletErrorKind_DiskFull() when diskFull != null:
return diskFull();case WalletErrorKind_StoreBusy() when storeBusy != null:
return storeBusy();case WalletErrorKind_WalletAlreadyOpen() when walletAlreadyOpen != null:
return walletAlreadyOpen();case WalletErrorKind_WalletBusy() when walletBusy != null:
return walletBusy(_that.phase);case WalletErrorKind_InvalidState() when invalidState != null:
return invalidState(_that.phase);case WalletErrorKind_WalletOpen() when walletOpen != null:
return walletOpen();case WalletErrorKind_SyncServerNotOffered() when syncServerNotOffered != null:
return syncServerNotOffered();case WalletErrorKind_SyncServerUnreachable() when syncServerUnreachable != null:
return syncServerUnreachable();case WalletErrorKind_WipeWithPendingSwap() when wipeWithPendingSwap != null:
return wipeWithPendingSwap(_that.count);case WalletErrorKind_RescanWithInFlightSend() when rescanWithInFlightSend != null:
return rescanWithInFlightSend();case WalletErrorKind_SwapAddressCheckRefused() when swapAddressCheckRefused != null:
return swapAddressCheckRefused(_that.reason);case WalletErrorKind_Sync() when sync_ != null:
return sync_(_that.stall);case WalletErrorKind_NetworkUpgradeUnsupported() when networkUpgradeUnsupported != null:
return networkUpgradeUnsupported(_that.expectedBranchId,_that.endpointBranchId,_that.judgedAtHeight);case WalletErrorKind_ConsensusGraceExpired() when consensusGraceExpired != null:
return consensusGraceExpired(_that.by,_that.blocksSinceLastCurrent);case WalletErrorKind_ConsensusNotEvaluated() when consensusNotEvaluated != null:
return consensusNotEvaluated();case WalletErrorKind_SyncRunning() when syncRunning != null:
return syncRunning();case WalletErrorKind_InsufficientFunds() when insufficientFunds != null:
return insufficientFunds(_that.availableZat,_that.requiredZat,_that.pendingIncomingZat);case WalletErrorKind_SendAmountRequired() when sendAmountRequired != null:
return sendAmountRequired();case WalletErrorKind_ProposalAlreadyUsed() when proposalAlreadyUsed != null:
return proposalAlreadyUsed();case WalletErrorKind_ProposalStale() when proposalStale != null:
return proposalStale();case WalletErrorKind_ProposeFailed() when proposeFailed != null:
return proposeFailed();case WalletErrorKind_ProposeTransient() when proposeTransient != null:
return proposeTransient();case WalletErrorKind_SignFailed() when signFailed != null:
return signFailed();case WalletErrorKind_QueuedSendStale() when queuedSendStale != null:
return queuedSendStale();case WalletErrorKind_QueuedSendsFull() when queuedSendsFull != null:
return queuedSendsFull();case WalletErrorKind_TexSendLimitReached() when texSendLimitReached != null:
return texSendLimitReached();case WalletErrorKind_Io() when io != null:
return io();case WalletErrorKind_Unknown() when unknown != null:
return unknown();case _:
  return null;

}
}

}

/// @nodoc


class WalletErrorKind_InvalidSeedLength extends WalletErrorKind {
  const WalletErrorKind_InvalidSeedLength({required this.len}): super._();
  

 final  BigInt len;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidSeedLengthCopyWith<WalletErrorKind_InvalidSeedLength> get copyWith => _$WalletErrorKind_InvalidSeedLengthCopyWithImpl<WalletErrorKind_InvalidSeedLength>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidSeedLength&&(identical(other.len, len) || other.len == len));
}


@override
int get hashCode => Object.hash(runtimeType,len);

@override
String toString() {
  return 'WalletErrorKind.invalidSeedLength(len: $len)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidSeedLengthCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidSeedLengthCopyWith(WalletErrorKind_InvalidSeedLength value, $Res Function(WalletErrorKind_InvalidSeedLength) _then) = _$WalletErrorKind_InvalidSeedLengthCopyWithImpl;
@useResult
$Res call({
 BigInt len
});




}
/// @nodoc
class _$WalletErrorKind_InvalidSeedLengthCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidSeedLengthCopyWith<$Res> {
  _$WalletErrorKind_InvalidSeedLengthCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidSeedLength _self;
  final $Res Function(WalletErrorKind_InvalidSeedLength) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? len = null,}) {
  return _then(WalletErrorKind_InvalidSeedLength(
len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletErrorKind_InvalidMnemonic extends WalletErrorKind {
  const WalletErrorKind_InvalidMnemonic({this.wordIndex}): super._();
  

 final  int? wordIndex;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidMnemonicCopyWith<WalletErrorKind_InvalidMnemonic> get copyWith => _$WalletErrorKind_InvalidMnemonicCopyWithImpl<WalletErrorKind_InvalidMnemonic>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidMnemonic&&(identical(other.wordIndex, wordIndex) || other.wordIndex == wordIndex));
}


@override
int get hashCode => Object.hash(runtimeType,wordIndex);

@override
String toString() {
  return 'WalletErrorKind.invalidMnemonic(wordIndex: $wordIndex)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidMnemonicCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidMnemonicCopyWith(WalletErrorKind_InvalidMnemonic value, $Res Function(WalletErrorKind_InvalidMnemonic) _then) = _$WalletErrorKind_InvalidMnemonicCopyWithImpl;
@useResult
$Res call({
 int? wordIndex
});




}
/// @nodoc
class _$WalletErrorKind_InvalidMnemonicCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidMnemonicCopyWith<$Res> {
  _$WalletErrorKind_InvalidMnemonicCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidMnemonic _self;
  final $Res Function(WalletErrorKind_InvalidMnemonic) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? wordIndex = freezed,}) {
  return _then(WalletErrorKind_InvalidMnemonic(
wordIndex: freezed == wordIndex ? _self.wordIndex : wordIndex // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class WalletErrorKind_NoMnemonic extends WalletErrorKind {
  const WalletErrorKind_NoMnemonic(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_NoMnemonic);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.noMnemonic()';
}


}




/// @nodoc


class WalletErrorKind_SeedRequired extends WalletErrorKind {
  const WalletErrorKind_SeedRequired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SeedRequired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.seedRequired()';
}


}




/// @nodoc


class WalletErrorKind_SeedMismatch extends WalletErrorKind {
  const WalletErrorKind_SeedMismatch(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SeedMismatch);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.seedMismatch()';
}


}




/// @nodoc


class WalletErrorKind_KeyDerivation extends WalletErrorKind {
  const WalletErrorKind_KeyDerivation(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_KeyDerivation);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.keyDerivation()';
}


}




/// @nodoc


class WalletErrorKind_WatchOnly extends WalletErrorKind {
  const WalletErrorKind_WatchOnly(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WatchOnly);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.watchOnly()';
}


}




/// @nodoc


class WalletErrorKind_InvalidViewingKey extends WalletErrorKind {
  const WalletErrorKind_InvalidViewingKey(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidViewingKey);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.invalidViewingKey()';
}


}




/// @nodoc


class WalletErrorKind_InvalidEndpoint extends WalletErrorKind {
  const WalletErrorKind_InvalidEndpoint({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidEndpointCopyWith<WalletErrorKind_InvalidEndpoint> get copyWith => _$WalletErrorKind_InvalidEndpointCopyWithImpl<WalletErrorKind_InvalidEndpoint>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidEndpoint&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'WalletErrorKind.invalidEndpoint(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidEndpointCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidEndpointCopyWith(WalletErrorKind_InvalidEndpoint value, $Res Function(WalletErrorKind_InvalidEndpoint) _then) = _$WalletErrorKind_InvalidEndpointCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$WalletErrorKind_InvalidEndpointCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidEndpointCopyWith<$Res> {
  _$WalletErrorKind_InvalidEndpointCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidEndpoint _self;
  final $Res Function(WalletErrorKind_InvalidEndpoint) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(WalletErrorKind_InvalidEndpoint(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletErrorKind_BirthdayInFuture extends WalletErrorKind {
  const WalletErrorKind_BirthdayInFuture(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_BirthdayInFuture);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.birthdayInFuture()';
}


}




/// @nodoc


class WalletErrorKind_InvalidDbDir extends WalletErrorKind {
  const WalletErrorKind_InvalidDbDir({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidDbDirCopyWith<WalletErrorKind_InvalidDbDir> get copyWith => _$WalletErrorKind_InvalidDbDirCopyWithImpl<WalletErrorKind_InvalidDbDir>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidDbDir&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'WalletErrorKind.invalidDbDir(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidDbDirCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidDbDirCopyWith(WalletErrorKind_InvalidDbDir value, $Res Function(WalletErrorKind_InvalidDbDir) _then) = _$WalletErrorKind_InvalidDbDirCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$WalletErrorKind_InvalidDbDirCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidDbDirCopyWith<$Res> {
  _$WalletErrorKind_InvalidDbDirCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidDbDir _self;
  final $Res Function(WalletErrorKind_InvalidDbDir) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(WalletErrorKind_InvalidDbDir(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletErrorKind_InvalidEndpointAuth extends WalletErrorKind {
  const WalletErrorKind_InvalidEndpointAuth({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidEndpointAuthCopyWith<WalletErrorKind_InvalidEndpointAuth> get copyWith => _$WalletErrorKind_InvalidEndpointAuthCopyWithImpl<WalletErrorKind_InvalidEndpointAuth>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidEndpointAuth&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'WalletErrorKind.invalidEndpointAuth(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidEndpointAuthCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidEndpointAuthCopyWith(WalletErrorKind_InvalidEndpointAuth value, $Res Function(WalletErrorKind_InvalidEndpointAuth) _then) = _$WalletErrorKind_InvalidEndpointAuthCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$WalletErrorKind_InvalidEndpointAuthCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidEndpointAuthCopyWith<$Res> {
  _$WalletErrorKind_InvalidEndpointAuthCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidEndpointAuth _self;
  final $Res Function(WalletErrorKind_InvalidEndpointAuth) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(WalletErrorKind_InvalidEndpointAuth(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletErrorKind_BroadcastJitterTooLong extends WalletErrorKind {
  const WalletErrorKind_BroadcastJitterTooLong({required this.maxMs, required this.ceilingMs}): super._();
  

 final  BigInt maxMs;
 final  BigInt ceilingMs;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_BroadcastJitterTooLongCopyWith<WalletErrorKind_BroadcastJitterTooLong> get copyWith => _$WalletErrorKind_BroadcastJitterTooLongCopyWithImpl<WalletErrorKind_BroadcastJitterTooLong>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_BroadcastJitterTooLong&&(identical(other.maxMs, maxMs) || other.maxMs == maxMs)&&(identical(other.ceilingMs, ceilingMs) || other.ceilingMs == ceilingMs));
}


@override
int get hashCode => Object.hash(runtimeType,maxMs,ceilingMs);

@override
String toString() {
  return 'WalletErrorKind.broadcastJitterTooLong(maxMs: $maxMs, ceilingMs: $ceilingMs)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_BroadcastJitterTooLongCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_BroadcastJitterTooLongCopyWith(WalletErrorKind_BroadcastJitterTooLong value, $Res Function(WalletErrorKind_BroadcastJitterTooLong) _then) = _$WalletErrorKind_BroadcastJitterTooLongCopyWithImpl;
@useResult
$Res call({
 BigInt maxMs, BigInt ceilingMs
});




}
/// @nodoc
class _$WalletErrorKind_BroadcastJitterTooLongCopyWithImpl<$Res>
    implements $WalletErrorKind_BroadcastJitterTooLongCopyWith<$Res> {
  _$WalletErrorKind_BroadcastJitterTooLongCopyWithImpl(this._self, this._then);

  final WalletErrorKind_BroadcastJitterTooLong _self;
  final $Res Function(WalletErrorKind_BroadcastJitterTooLong) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? maxMs = null,Object? ceilingMs = null,}) {
  return _then(WalletErrorKind_BroadcastJitterTooLong(
maxMs: null == maxMs ? _self.maxMs : maxMs // ignore: cast_nullable_to_non_nullable
as BigInt,ceilingMs: null == ceilingMs ? _self.ceilingMs : ceilingMs // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletErrorKind_AmountOutOfRange extends WalletErrorKind {
  const WalletErrorKind_AmountOutOfRange(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_AmountOutOfRange);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.amountOutOfRange()';
}


}




/// @nodoc


class WalletErrorKind_MemoTooLong extends WalletErrorKind {
  const WalletErrorKind_MemoTooLong({required this.len, required this.max}): super._();
  

 final  BigInt len;
 final  BigInt max;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_MemoTooLongCopyWith<WalletErrorKind_MemoTooLong> get copyWith => _$WalletErrorKind_MemoTooLongCopyWithImpl<WalletErrorKind_MemoTooLong>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_MemoTooLong&&(identical(other.len, len) || other.len == len)&&(identical(other.max, max) || other.max == max));
}


@override
int get hashCode => Object.hash(runtimeType,len,max);

@override
String toString() {
  return 'WalletErrorKind.memoTooLong(len: $len, max: $max)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_MemoTooLongCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_MemoTooLongCopyWith(WalletErrorKind_MemoTooLong value, $Res Function(WalletErrorKind_MemoTooLong) _then) = _$WalletErrorKind_MemoTooLongCopyWithImpl;
@useResult
$Res call({
 BigInt len, BigInt max
});




}
/// @nodoc
class _$WalletErrorKind_MemoTooLongCopyWithImpl<$Res>
    implements $WalletErrorKind_MemoTooLongCopyWith<$Res> {
  _$WalletErrorKind_MemoTooLongCopyWithImpl(this._self, this._then);

  final WalletErrorKind_MemoTooLong _self;
  final $Res Function(WalletErrorKind_MemoTooLong) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? len = null,Object? max = null,}) {
  return _then(WalletErrorKind_MemoTooLong(
len: null == len ? _self.len : len // ignore: cast_nullable_to_non_nullable
as BigInt,max: null == max ? _self.max : max // ignore: cast_nullable_to_non_nullable
as BigInt,
  ));
}


}

/// @nodoc


class WalletErrorKind_ReservedMemoNotSendable extends WalletErrorKind {
  const WalletErrorKind_ReservedMemoNotSendable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ReservedMemoNotSendable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.reservedMemoNotSendable()';
}


}




/// @nodoc


class WalletErrorKind_MemoRequiresShieldedRecipient extends WalletErrorKind {
  const WalletErrorKind_MemoRequiresShieldedRecipient(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_MemoRequiresShieldedRecipient);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.memoRequiresShieldedRecipient()';
}


}




/// @nodoc


class WalletErrorKind_AddressInvalid extends WalletErrorKind {
  const WalletErrorKind_AddressInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_AddressInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.addressInvalid()';
}


}




/// @nodoc


class WalletErrorKind_MemoInvalid extends WalletErrorKind {
  const WalletErrorKind_MemoInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_MemoInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.memoInvalid()';
}


}




/// @nodoc


class WalletErrorKind_MemoConflict extends WalletErrorKind {
  const WalletErrorKind_MemoConflict(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_MemoConflict);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.memoConflict()';
}


}




/// @nodoc


class WalletErrorKind_ZeroValuedTransparentOutput extends WalletErrorKind {
  const WalletErrorKind_ZeroValuedTransparentOutput(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ZeroValuedTransparentOutput);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.zeroValuedTransparentOutput()';
}


}




/// @nodoc


class WalletErrorKind_PaymentUriInvalid extends WalletErrorKind {
  const WalletErrorKind_PaymentUriInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_PaymentUriInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.paymentUriInvalid()';
}


}




/// @nodoc


class WalletErrorKind_TxidInvalid extends WalletErrorKind {
  const WalletErrorKind_TxidInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_TxidInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.txidInvalid()';
}


}




/// @nodoc


class WalletErrorKind_MachineMemoScopeInvalid extends WalletErrorKind {
  const WalletErrorKind_MachineMemoScopeInvalid({required this.reason}): super._();
  

 final  String reason;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_MachineMemoScopeInvalidCopyWith<WalletErrorKind_MachineMemoScopeInvalid> get copyWith => _$WalletErrorKind_MachineMemoScopeInvalidCopyWithImpl<WalletErrorKind_MachineMemoScopeInvalid>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_MachineMemoScopeInvalid&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'WalletErrorKind.machineMemoScopeInvalid(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_MachineMemoScopeInvalidCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_MachineMemoScopeInvalidCopyWith(WalletErrorKind_MachineMemoScopeInvalid value, $Res Function(WalletErrorKind_MachineMemoScopeInvalid) _then) = _$WalletErrorKind_MachineMemoScopeInvalidCopyWithImpl;
@useResult
$Res call({
 String reason
});




}
/// @nodoc
class _$WalletErrorKind_MachineMemoScopeInvalidCopyWithImpl<$Res>
    implements $WalletErrorKind_MachineMemoScopeInvalidCopyWith<$Res> {
  _$WalletErrorKind_MachineMemoScopeInvalidCopyWithImpl(this._self, this._then);

  final WalletErrorKind_MachineMemoScopeInvalid _self;
  final $Res Function(WalletErrorKind_MachineMemoScopeInvalid) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(WalletErrorKind_MachineMemoScopeInvalid(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as String,
  ));
}


}

/// @nodoc


class WalletErrorKind_KeystoreUnavailable extends WalletErrorKind {
  const WalletErrorKind_KeystoreUnavailable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_KeystoreUnavailable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.keystoreUnavailable()';
}


}




/// @nodoc


class WalletErrorKind_KeystoreInconsistent extends WalletErrorKind {
  const WalletErrorKind_KeystoreInconsistent({required this.permanentlyInvalidated}): super._();
  

 final  bool permanentlyInvalidated;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_KeystoreInconsistentCopyWith<WalletErrorKind_KeystoreInconsistent> get copyWith => _$WalletErrorKind_KeystoreInconsistentCopyWithImpl<WalletErrorKind_KeystoreInconsistent>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_KeystoreInconsistent&&(identical(other.permanentlyInvalidated, permanentlyInvalidated) || other.permanentlyInvalidated == permanentlyInvalidated));
}


@override
int get hashCode => Object.hash(runtimeType,permanentlyInvalidated);

@override
String toString() {
  return 'WalletErrorKind.keystoreInconsistent(permanentlyInvalidated: $permanentlyInvalidated)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_KeystoreInconsistentCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_KeystoreInconsistentCopyWith(WalletErrorKind_KeystoreInconsistent value, $Res Function(WalletErrorKind_KeystoreInconsistent) _then) = _$WalletErrorKind_KeystoreInconsistentCopyWithImpl;
@useResult
$Res call({
 bool permanentlyInvalidated
});




}
/// @nodoc
class _$WalletErrorKind_KeystoreInconsistentCopyWithImpl<$Res>
    implements $WalletErrorKind_KeystoreInconsistentCopyWith<$Res> {
  _$WalletErrorKind_KeystoreInconsistentCopyWithImpl(this._self, this._then);

  final WalletErrorKind_KeystoreInconsistent _self;
  final $Res Function(WalletErrorKind_KeystoreInconsistent) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? permanentlyInvalidated = null,}) {
  return _then(WalletErrorKind_KeystoreInconsistent(
permanentlyInvalidated: null == permanentlyInvalidated ? _self.permanentlyInvalidated : permanentlyInvalidated // ignore: cast_nullable_to_non_nullable
as bool,
  ));
}


}

/// @nodoc


class WalletErrorKind_SealVersionUnsupported extends WalletErrorKind {
  const WalletErrorKind_SealVersionUnsupported({required this.found}): super._();
  

 final  int found;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_SealVersionUnsupportedCopyWith<WalletErrorKind_SealVersionUnsupported> get copyWith => _$WalletErrorKind_SealVersionUnsupportedCopyWithImpl<WalletErrorKind_SealVersionUnsupported>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SealVersionUnsupported&&(identical(other.found, found) || other.found == found));
}


@override
int get hashCode => Object.hash(runtimeType,found);

@override
String toString() {
  return 'WalletErrorKind.sealVersionUnsupported(found: $found)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_SealVersionUnsupportedCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_SealVersionUnsupportedCopyWith(WalletErrorKind_SealVersionUnsupported value, $Res Function(WalletErrorKind_SealVersionUnsupported) _then) = _$WalletErrorKind_SealVersionUnsupportedCopyWithImpl;
@useResult
$Res call({
 int found
});




}
/// @nodoc
class _$WalletErrorKind_SealVersionUnsupportedCopyWithImpl<$Res>
    implements $WalletErrorKind_SealVersionUnsupportedCopyWith<$Res> {
  _$WalletErrorKind_SealVersionUnsupportedCopyWithImpl(this._self, this._then);

  final WalletErrorKind_SealVersionUnsupported _self;
  final $Res Function(WalletErrorKind_SealVersionUnsupported) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? found = null,}) {
  return _then(WalletErrorKind_SealVersionUnsupported(
found: null == found ? _self.found : found // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class WalletErrorKind_SealInvalid extends WalletErrorKind {
  const WalletErrorKind_SealInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SealInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.sealInvalid()';
}


}




/// @nodoc


class WalletErrorKind_VaultAbsent extends WalletErrorKind {
  const WalletErrorKind_VaultAbsent(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_VaultAbsent);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.vaultAbsent()';
}


}




/// @nodoc


class WalletErrorKind_WrapArtifactInvalid extends WalletErrorKind {
  const WalletErrorKind_WrapArtifactInvalid(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WrapArtifactInvalid);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.wrapArtifactInvalid()';
}


}




/// @nodoc


class WalletErrorKind_WrapVersionUnsupported extends WalletErrorKind {
  const WalletErrorKind_WrapVersionUnsupported({required this.found}): super._();
  

 final  int found;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_WrapVersionUnsupportedCopyWith<WalletErrorKind_WrapVersionUnsupported> get copyWith => _$WalletErrorKind_WrapVersionUnsupportedCopyWithImpl<WalletErrorKind_WrapVersionUnsupported>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WrapVersionUnsupported&&(identical(other.found, found) || other.found == found));
}


@override
int get hashCode => Object.hash(runtimeType,found);

@override
String toString() {
  return 'WalletErrorKind.wrapVersionUnsupported(found: $found)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_WrapVersionUnsupportedCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_WrapVersionUnsupportedCopyWith(WalletErrorKind_WrapVersionUnsupported value, $Res Function(WalletErrorKind_WrapVersionUnsupported) _then) = _$WalletErrorKind_WrapVersionUnsupportedCopyWithImpl;
@useResult
$Res call({
 int found
});




}
/// @nodoc
class _$WalletErrorKind_WrapVersionUnsupportedCopyWithImpl<$Res>
    implements $WalletErrorKind_WrapVersionUnsupportedCopyWith<$Res> {
  _$WalletErrorKind_WrapVersionUnsupportedCopyWithImpl(this._self, this._then);

  final WalletErrorKind_WrapVersionUnsupported _self;
  final $Res Function(WalletErrorKind_WrapVersionUnsupported) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? found = null,}) {
  return _then(WalletErrorKind_WrapVersionUnsupported(
found: null == found ? _self.found : found // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class WalletErrorKind_NotFound extends WalletErrorKind {
  const WalletErrorKind_NotFound(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_NotFound);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.notFound()';
}


}




/// @nodoc


class WalletErrorKind_NetworkMismatch extends WalletErrorKind {
  const WalletErrorKind_NetworkMismatch(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_NetworkMismatch);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.networkMismatch()';
}


}




/// @nodoc


class WalletErrorKind_ProvisioningIncomplete extends WalletErrorKind {
  const WalletErrorKind_ProvisioningIncomplete(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ProvisioningIncomplete);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.provisioningIncomplete()';
}


}




/// @nodoc


class WalletErrorKind_WalletAlreadyExists extends WalletErrorKind {
  const WalletErrorKind_WalletAlreadyExists(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WalletAlreadyExists);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.walletAlreadyExists()';
}


}




/// @nodoc


class WalletErrorKind_StoreCorrupt extends WalletErrorKind {
  const WalletErrorKind_StoreCorrupt(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_StoreCorrupt);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.storeCorrupt()';
}


}




/// @nodoc


class WalletErrorKind_DiskFull extends WalletErrorKind {
  const WalletErrorKind_DiskFull(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_DiskFull);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.diskFull()';
}


}




/// @nodoc


class WalletErrorKind_StoreBusy extends WalletErrorKind {
  const WalletErrorKind_StoreBusy(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_StoreBusy);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.storeBusy()';
}


}




/// @nodoc


class WalletErrorKind_WalletAlreadyOpen extends WalletErrorKind {
  const WalletErrorKind_WalletAlreadyOpen(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WalletAlreadyOpen);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.walletAlreadyOpen()';
}


}




/// @nodoc


class WalletErrorKind_WalletBusy extends WalletErrorKind {
  const WalletErrorKind_WalletBusy({required this.phase}): super._();
  

 final  LifecyclePhase phase;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_WalletBusyCopyWith<WalletErrorKind_WalletBusy> get copyWith => _$WalletErrorKind_WalletBusyCopyWithImpl<WalletErrorKind_WalletBusy>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WalletBusy&&(identical(other.phase, phase) || other.phase == phase));
}


@override
int get hashCode => Object.hash(runtimeType,phase);

@override
String toString() {
  return 'WalletErrorKind.walletBusy(phase: $phase)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_WalletBusyCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_WalletBusyCopyWith(WalletErrorKind_WalletBusy value, $Res Function(WalletErrorKind_WalletBusy) _then) = _$WalletErrorKind_WalletBusyCopyWithImpl;
@useResult
$Res call({
 LifecyclePhase phase
});




}
/// @nodoc
class _$WalletErrorKind_WalletBusyCopyWithImpl<$Res>
    implements $WalletErrorKind_WalletBusyCopyWith<$Res> {
  _$WalletErrorKind_WalletBusyCopyWithImpl(this._self, this._then);

  final WalletErrorKind_WalletBusy _self;
  final $Res Function(WalletErrorKind_WalletBusy) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? phase = null,}) {
  return _then(WalletErrorKind_WalletBusy(
phase: null == phase ? _self.phase : phase // ignore: cast_nullable_to_non_nullable
as LifecyclePhase,
  ));
}


}

/// @nodoc


class WalletErrorKind_InvalidState extends WalletErrorKind {
  const WalletErrorKind_InvalidState({required this.phase}): super._();
  

 final  LifecyclePhase phase;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InvalidStateCopyWith<WalletErrorKind_InvalidState> get copyWith => _$WalletErrorKind_InvalidStateCopyWithImpl<WalletErrorKind_InvalidState>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InvalidState&&(identical(other.phase, phase) || other.phase == phase));
}


@override
int get hashCode => Object.hash(runtimeType,phase);

@override
String toString() {
  return 'WalletErrorKind.invalidState(phase: $phase)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InvalidStateCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InvalidStateCopyWith(WalletErrorKind_InvalidState value, $Res Function(WalletErrorKind_InvalidState) _then) = _$WalletErrorKind_InvalidStateCopyWithImpl;
@useResult
$Res call({
 LifecyclePhase phase
});




}
/// @nodoc
class _$WalletErrorKind_InvalidStateCopyWithImpl<$Res>
    implements $WalletErrorKind_InvalidStateCopyWith<$Res> {
  _$WalletErrorKind_InvalidStateCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InvalidState _self;
  final $Res Function(WalletErrorKind_InvalidState) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? phase = null,}) {
  return _then(WalletErrorKind_InvalidState(
phase: null == phase ? _self.phase : phase // ignore: cast_nullable_to_non_nullable
as LifecyclePhase,
  ));
}


}

/// @nodoc


class WalletErrorKind_WalletOpen extends WalletErrorKind {
  const WalletErrorKind_WalletOpen(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WalletOpen);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.walletOpen()';
}


}




/// @nodoc


class WalletErrorKind_SyncServerNotOffered extends WalletErrorKind {
  const WalletErrorKind_SyncServerNotOffered(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SyncServerNotOffered);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.syncServerNotOffered()';
}


}




/// @nodoc


class WalletErrorKind_SyncServerUnreachable extends WalletErrorKind {
  const WalletErrorKind_SyncServerUnreachable(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SyncServerUnreachable);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.syncServerUnreachable()';
}


}




/// @nodoc


class WalletErrorKind_WipeWithPendingSwap extends WalletErrorKind {
  const WalletErrorKind_WipeWithPendingSwap({required this.count}): super._();
  

 final  int count;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_WipeWithPendingSwapCopyWith<WalletErrorKind_WipeWithPendingSwap> get copyWith => _$WalletErrorKind_WipeWithPendingSwapCopyWithImpl<WalletErrorKind_WipeWithPendingSwap>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_WipeWithPendingSwap&&(identical(other.count, count) || other.count == count));
}


@override
int get hashCode => Object.hash(runtimeType,count);

@override
String toString() {
  return 'WalletErrorKind.wipeWithPendingSwap(count: $count)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_WipeWithPendingSwapCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_WipeWithPendingSwapCopyWith(WalletErrorKind_WipeWithPendingSwap value, $Res Function(WalletErrorKind_WipeWithPendingSwap) _then) = _$WalletErrorKind_WipeWithPendingSwapCopyWithImpl;
@useResult
$Res call({
 int count
});




}
/// @nodoc
class _$WalletErrorKind_WipeWithPendingSwapCopyWithImpl<$Res>
    implements $WalletErrorKind_WipeWithPendingSwapCopyWith<$Res> {
  _$WalletErrorKind_WipeWithPendingSwapCopyWithImpl(this._self, this._then);

  final WalletErrorKind_WipeWithPendingSwap _self;
  final $Res Function(WalletErrorKind_WipeWithPendingSwap) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? count = null,}) {
  return _then(WalletErrorKind_WipeWithPendingSwap(
count: null == count ? _self.count : count // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class WalletErrorKind_RescanWithInFlightSend extends WalletErrorKind {
  const WalletErrorKind_RescanWithInFlightSend(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_RescanWithInFlightSend);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.rescanWithInFlightSend()';
}


}




/// @nodoc


class WalletErrorKind_SwapAddressCheckRefused extends WalletErrorKind {
  const WalletErrorKind_SwapAddressCheckRefused({required this.reason}): super._();
  

 final  SwapAddressCheckRefusal reason;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_SwapAddressCheckRefusedCopyWith<WalletErrorKind_SwapAddressCheckRefused> get copyWith => _$WalletErrorKind_SwapAddressCheckRefusedCopyWithImpl<WalletErrorKind_SwapAddressCheckRefused>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SwapAddressCheckRefused&&(identical(other.reason, reason) || other.reason == reason));
}


@override
int get hashCode => Object.hash(runtimeType,reason);

@override
String toString() {
  return 'WalletErrorKind.swapAddressCheckRefused(reason: $reason)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_SwapAddressCheckRefusedCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_SwapAddressCheckRefusedCopyWith(WalletErrorKind_SwapAddressCheckRefused value, $Res Function(WalletErrorKind_SwapAddressCheckRefused) _then) = _$WalletErrorKind_SwapAddressCheckRefusedCopyWithImpl;
@useResult
$Res call({
 SwapAddressCheckRefusal reason
});




}
/// @nodoc
class _$WalletErrorKind_SwapAddressCheckRefusedCopyWithImpl<$Res>
    implements $WalletErrorKind_SwapAddressCheckRefusedCopyWith<$Res> {
  _$WalletErrorKind_SwapAddressCheckRefusedCopyWithImpl(this._self, this._then);

  final WalletErrorKind_SwapAddressCheckRefused _self;
  final $Res Function(WalletErrorKind_SwapAddressCheckRefused) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? reason = null,}) {
  return _then(WalletErrorKind_SwapAddressCheckRefused(
reason: null == reason ? _self.reason : reason // ignore: cast_nullable_to_non_nullable
as SwapAddressCheckRefusal,
  ));
}


}

/// @nodoc


class WalletErrorKind_Sync extends WalletErrorKind {
  const WalletErrorKind_Sync({required this.stall}): super._();
  

 final  StallReason stall;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_SyncCopyWith<WalletErrorKind_Sync> get copyWith => _$WalletErrorKind_SyncCopyWithImpl<WalletErrorKind_Sync>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_Sync&&(identical(other.stall, stall) || other.stall == stall));
}


@override
int get hashCode => Object.hash(runtimeType,stall);

@override
String toString() {
  return 'WalletErrorKind.sync_(stall: $stall)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_SyncCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_SyncCopyWith(WalletErrorKind_Sync value, $Res Function(WalletErrorKind_Sync) _then) = _$WalletErrorKind_SyncCopyWithImpl;
@useResult
$Res call({
 StallReason stall
});




}
/// @nodoc
class _$WalletErrorKind_SyncCopyWithImpl<$Res>
    implements $WalletErrorKind_SyncCopyWith<$Res> {
  _$WalletErrorKind_SyncCopyWithImpl(this._self, this._then);

  final WalletErrorKind_Sync _self;
  final $Res Function(WalletErrorKind_Sync) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? stall = null,}) {
  return _then(WalletErrorKind_Sync(
stall: null == stall ? _self.stall : stall // ignore: cast_nullable_to_non_nullable
as StallReason,
  ));
}


}

/// @nodoc


class WalletErrorKind_NetworkUpgradeUnsupported extends WalletErrorKind {
  const WalletErrorKind_NetworkUpgradeUnsupported({required this.expectedBranchId, this.endpointBranchId, required this.judgedAtHeight}): super._();
  

 final  int expectedBranchId;
 final  int? endpointBranchId;
 final  int judgedAtHeight;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_NetworkUpgradeUnsupportedCopyWith<WalletErrorKind_NetworkUpgradeUnsupported> get copyWith => _$WalletErrorKind_NetworkUpgradeUnsupportedCopyWithImpl<WalletErrorKind_NetworkUpgradeUnsupported>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_NetworkUpgradeUnsupported&&(identical(other.expectedBranchId, expectedBranchId) || other.expectedBranchId == expectedBranchId)&&(identical(other.endpointBranchId, endpointBranchId) || other.endpointBranchId == endpointBranchId)&&(identical(other.judgedAtHeight, judgedAtHeight) || other.judgedAtHeight == judgedAtHeight));
}


@override
int get hashCode => Object.hash(runtimeType,expectedBranchId,endpointBranchId,judgedAtHeight);

@override
String toString() {
  return 'WalletErrorKind.networkUpgradeUnsupported(expectedBranchId: $expectedBranchId, endpointBranchId: $endpointBranchId, judgedAtHeight: $judgedAtHeight)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_NetworkUpgradeUnsupportedCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_NetworkUpgradeUnsupportedCopyWith(WalletErrorKind_NetworkUpgradeUnsupported value, $Res Function(WalletErrorKind_NetworkUpgradeUnsupported) _then) = _$WalletErrorKind_NetworkUpgradeUnsupportedCopyWithImpl;
@useResult
$Res call({
 int expectedBranchId, int? endpointBranchId, int judgedAtHeight
});




}
/// @nodoc
class _$WalletErrorKind_NetworkUpgradeUnsupportedCopyWithImpl<$Res>
    implements $WalletErrorKind_NetworkUpgradeUnsupportedCopyWith<$Res> {
  _$WalletErrorKind_NetworkUpgradeUnsupportedCopyWithImpl(this._self, this._then);

  final WalletErrorKind_NetworkUpgradeUnsupported _self;
  final $Res Function(WalletErrorKind_NetworkUpgradeUnsupported) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? expectedBranchId = null,Object? endpointBranchId = freezed,Object? judgedAtHeight = null,}) {
  return _then(WalletErrorKind_NetworkUpgradeUnsupported(
expectedBranchId: null == expectedBranchId ? _self.expectedBranchId : expectedBranchId // ignore: cast_nullable_to_non_nullable
as int,endpointBranchId: freezed == endpointBranchId ? _self.endpointBranchId : endpointBranchId // ignore: cast_nullable_to_non_nullable
as int?,judgedAtHeight: null == judgedAtHeight ? _self.judgedAtHeight : judgedAtHeight // ignore: cast_nullable_to_non_nullable
as int,
  ));
}


}

/// @nodoc


class WalletErrorKind_ConsensusGraceExpired extends WalletErrorKind {
  const WalletErrorKind_ConsensusGraceExpired({required this.by, this.blocksSinceLastCurrent}): super._();
  

 final  GraceExpiry by;
 final  int? blocksSinceLastCurrent;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_ConsensusGraceExpiredCopyWith<WalletErrorKind_ConsensusGraceExpired> get copyWith => _$WalletErrorKind_ConsensusGraceExpiredCopyWithImpl<WalletErrorKind_ConsensusGraceExpired>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ConsensusGraceExpired&&(identical(other.by, by) || other.by == by)&&(identical(other.blocksSinceLastCurrent, blocksSinceLastCurrent) || other.blocksSinceLastCurrent == blocksSinceLastCurrent));
}


@override
int get hashCode => Object.hash(runtimeType,by,blocksSinceLastCurrent);

@override
String toString() {
  return 'WalletErrorKind.consensusGraceExpired(by: $by, blocksSinceLastCurrent: $blocksSinceLastCurrent)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_ConsensusGraceExpiredCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_ConsensusGraceExpiredCopyWith(WalletErrorKind_ConsensusGraceExpired value, $Res Function(WalletErrorKind_ConsensusGraceExpired) _then) = _$WalletErrorKind_ConsensusGraceExpiredCopyWithImpl;
@useResult
$Res call({
 GraceExpiry by, int? blocksSinceLastCurrent
});




}
/// @nodoc
class _$WalletErrorKind_ConsensusGraceExpiredCopyWithImpl<$Res>
    implements $WalletErrorKind_ConsensusGraceExpiredCopyWith<$Res> {
  _$WalletErrorKind_ConsensusGraceExpiredCopyWithImpl(this._self, this._then);

  final WalletErrorKind_ConsensusGraceExpired _self;
  final $Res Function(WalletErrorKind_ConsensusGraceExpired) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? by = null,Object? blocksSinceLastCurrent = freezed,}) {
  return _then(WalletErrorKind_ConsensusGraceExpired(
by: null == by ? _self.by : by // ignore: cast_nullable_to_non_nullable
as GraceExpiry,blocksSinceLastCurrent: freezed == blocksSinceLastCurrent ? _self.blocksSinceLastCurrent : blocksSinceLastCurrent // ignore: cast_nullable_to_non_nullable
as int?,
  ));
}


}

/// @nodoc


class WalletErrorKind_ConsensusNotEvaluated extends WalletErrorKind {
  const WalletErrorKind_ConsensusNotEvaluated(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ConsensusNotEvaluated);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.consensusNotEvaluated()';
}


}




/// @nodoc


class WalletErrorKind_SyncRunning extends WalletErrorKind {
  const WalletErrorKind_SyncRunning(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SyncRunning);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.syncRunning()';
}


}




/// @nodoc


class WalletErrorKind_InsufficientFunds extends WalletErrorKind {
  const WalletErrorKind_InsufficientFunds({required this.availableZat, required this.requiredZat, required this.pendingIncomingZat}): super._();
  

 final  PlatformInt64 availableZat;
 final  PlatformInt64 requiredZat;
 final  PlatformInt64 pendingIncomingZat;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@JsonKey(includeFromJson: false, includeToJson: false)
@pragma('vm:prefer-inline')
$WalletErrorKind_InsufficientFundsCopyWith<WalletErrorKind_InsufficientFunds> get copyWith => _$WalletErrorKind_InsufficientFundsCopyWithImpl<WalletErrorKind_InsufficientFunds>(this, _$identity);



@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_InsufficientFunds&&(identical(other.availableZat, availableZat) || other.availableZat == availableZat)&&(identical(other.requiredZat, requiredZat) || other.requiredZat == requiredZat)&&(identical(other.pendingIncomingZat, pendingIncomingZat) || other.pendingIncomingZat == pendingIncomingZat));
}


@override
int get hashCode => Object.hash(runtimeType,availableZat,requiredZat,pendingIncomingZat);

@override
String toString() {
  return 'WalletErrorKind.insufficientFunds(availableZat: $availableZat, requiredZat: $requiredZat, pendingIncomingZat: $pendingIncomingZat)';
}


}

/// @nodoc
abstract mixin class $WalletErrorKind_InsufficientFundsCopyWith<$Res> implements $WalletErrorKindCopyWith<$Res> {
  factory $WalletErrorKind_InsufficientFundsCopyWith(WalletErrorKind_InsufficientFunds value, $Res Function(WalletErrorKind_InsufficientFunds) _then) = _$WalletErrorKind_InsufficientFundsCopyWithImpl;
@useResult
$Res call({
 PlatformInt64 availableZat, PlatformInt64 requiredZat, PlatformInt64 pendingIncomingZat
});




}
/// @nodoc
class _$WalletErrorKind_InsufficientFundsCopyWithImpl<$Res>
    implements $WalletErrorKind_InsufficientFundsCopyWith<$Res> {
  _$WalletErrorKind_InsufficientFundsCopyWithImpl(this._self, this._then);

  final WalletErrorKind_InsufficientFunds _self;
  final $Res Function(WalletErrorKind_InsufficientFunds) _then;

/// Create a copy of WalletErrorKind
/// with the given fields replaced by the non-null parameter values.
@pragma('vm:prefer-inline') $Res call({Object? availableZat = null,Object? requiredZat = null,Object? pendingIncomingZat = null,}) {
  return _then(WalletErrorKind_InsufficientFunds(
availableZat: null == availableZat ? _self.availableZat : availableZat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,requiredZat: null == requiredZat ? _self.requiredZat : requiredZat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,pendingIncomingZat: null == pendingIncomingZat ? _self.pendingIncomingZat : pendingIncomingZat // ignore: cast_nullable_to_non_nullable
as PlatformInt64,
  ));
}


}

/// @nodoc


class WalletErrorKind_SendAmountRequired extends WalletErrorKind {
  const WalletErrorKind_SendAmountRequired(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SendAmountRequired);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.sendAmountRequired()';
}


}




/// @nodoc


class WalletErrorKind_ProposalAlreadyUsed extends WalletErrorKind {
  const WalletErrorKind_ProposalAlreadyUsed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ProposalAlreadyUsed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.proposalAlreadyUsed()';
}


}




/// @nodoc


class WalletErrorKind_ProposalStale extends WalletErrorKind {
  const WalletErrorKind_ProposalStale(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ProposalStale);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.proposalStale()';
}


}




/// @nodoc


class WalletErrorKind_ProposeFailed extends WalletErrorKind {
  const WalletErrorKind_ProposeFailed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ProposeFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.proposeFailed()';
}


}




/// @nodoc


class WalletErrorKind_ProposeTransient extends WalletErrorKind {
  const WalletErrorKind_ProposeTransient(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_ProposeTransient);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.proposeTransient()';
}


}




/// @nodoc


class WalletErrorKind_SignFailed extends WalletErrorKind {
  const WalletErrorKind_SignFailed(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_SignFailed);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.signFailed()';
}


}




/// @nodoc


class WalletErrorKind_QueuedSendStale extends WalletErrorKind {
  const WalletErrorKind_QueuedSendStale(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_QueuedSendStale);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.queuedSendStale()';
}


}




/// @nodoc


class WalletErrorKind_QueuedSendsFull extends WalletErrorKind {
  const WalletErrorKind_QueuedSendsFull(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_QueuedSendsFull);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.queuedSendsFull()';
}


}




/// @nodoc


class WalletErrorKind_TexSendLimitReached extends WalletErrorKind {
  const WalletErrorKind_TexSendLimitReached(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_TexSendLimitReached);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.texSendLimitReached()';
}


}




/// @nodoc


class WalletErrorKind_Io extends WalletErrorKind {
  const WalletErrorKind_Io(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_Io);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.io()';
}


}




/// @nodoc


class WalletErrorKind_Unknown extends WalletErrorKind {
  const WalletErrorKind_Unknown(): super._();
  






@override
bool operator ==(Object other) {
  return identical(this, other) || (other.runtimeType == runtimeType&&other is WalletErrorKind_Unknown);
}


@override
int get hashCode => runtimeType.hashCode;

@override
String toString() {
  return 'WalletErrorKind.unknown()';
}


}




// dart format on
