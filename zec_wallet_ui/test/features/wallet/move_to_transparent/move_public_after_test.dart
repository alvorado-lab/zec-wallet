import 'package:flutter_test/flutter_test.dart';
import 'package:zec_wallet_ui/features/wallet/move_to_transparent/move_public_after.dart';

/// Stage S14 (`stage-14-move-below-the-shield-floor.md` §3): the figure the
/// Move review's floor warning quotes, and the floor test on it.
void main() {
  test('the moved amount adds to what is already public', () {
    expect(
      movePublicAfterZat(
        transparentZat: 60000,
        recoverableZat: 0,
        movedZat: 50000,
      ),
      110000,
    );
  });

  test('the one-time-address subset comes off first, clamped to the public '
      'balance (the two providers can skew)', () {
    expect(
      movePublicAfterZat(
        transparentZat: 80000,
        recoverableZat: 30000,
        movedZat: 50000,
      ),
      100000,
    );
    expect(
      movePublicAfterZat(
        transparentZat: 20000,
        recoverableZat: 90000,
        movedZat: 50000,
      ),
      50000,
      reason: 'a stale-high recoverable list never drives the figure negative',
    );
  });

  test('an unknown public balance (snapshot loading) counts as nothing, so '
      'the warning errs toward showing', () {
    expect(
      movePublicAfterZat(
        transparentZat: null,
        recoverableZat: 0,
        movedZat: 50000,
      ),
      50000,
    );
  });

  test('an unread or failed recoverable list counts as no one-time funds, as '
      'the auto-shield loop assumes', () {
    expect(
      movePublicAfterZat(
        transparentZat: 80000,
        recoverableZat: null,
        movedZat: 50000,
      ),
      130000,
    );
  });

  test('the loop\'s own figure is the raw public balance plus the amount', () {
    expect(
      moveRawPublicAfterZat(transparentZat: 80000, movedZat: 50000),
      130000,
    );
    expect(moveRawPublicAfterZat(transparentZat: null, movedZat: 50000), 50000);
  });

  test('the floor is the core\'s 100 000 zat, compared strictly', () {
    expect(moveLeavesPublicBelowFloor(99999), isTrue);
    expect(moveLeavesPublicBelowFloor(100000), isFalse);
  });

  test('the auto-shield promise needs the loop on AND both bounds met', () {
    bool promise(int after, {int? raw, bool on = true, int host = 100000}) =>
        moveReviewPromisesAutoShield(
          publicAfterZat: after,
          rawPublicAfterZat: raw ?? after,
          autoShieldEffective: on,
          hostThresholdZat: host,
        );
    expect(promise(100000), isTrue);
    expect(promise(99999, host: 0), isFalse, reason: 'under the core floor');
    expect(promise(150000, host: 200000), isFalse, reason: 'under the host');
    expect(promise(200000, on: false), isFalse, reason: 'the loop is off');
    expect(
      promise(150000, raw: 250000, host: 200000),
      isTrue,
      reason: 'the loop fires on the RAW public balance, as its own gate does',
    );
  });
}
