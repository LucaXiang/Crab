/**
 * Member-related order commands: linkMember, unlinkMember, redeemStamp.
 */

import { createCommand } from '../commandUtils';
import { sendCommand, ensureSuccess } from './sendCommand';

/**
 * Link a member to an order.
 * The backend will look up member info and apply MG discounts.
 */
export const linkMember = async (
  orderId: string,
  memberId: number,
): Promise<void> => {
  const command = createCommand({
    type: 'LINK_MEMBER',
    order_id: orderId,
    member_id: memberId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Link member');
};

/**
 * Unlink a member from an order.
 * The backend will remove MG discounts and recalculate totals.
 */
export const unlinkMember = async (
  orderId: string,
): Promise<void> => {
  const command = createCommand({
    type: 'UNLINK_MEMBER',
    order_id: orderId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Unlink member');
};

/**
 * Redeem a stamp reward for an order.
 * The backend will comp the appropriate item based on the activity's reward strategy.
 */
export const redeemStamp = async (
  orderId: string,
  stampActivityId: number,
  productId?: number | null,
  compExistingInstanceId?: string | null,
): Promise<void> => {
  const command = createCommand({
    type: 'REDEEM_STAMP',
    order_id: orderId,
    stamp_activity_id: stampActivityId,
    product_id: productId ?? null,
    comp_existing_instance_id: compExistingInstanceId ?? null,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Redeem stamp');
};

/**
 * Cancel a stamp redemption for an order.
 * Removes the reward item and allows re-redemption.
 */
export const cancelStampRedemption = async (
  orderId: string,
  stampActivityId: number,
): Promise<void> => {
  const command = createCommand({
    type: 'CANCEL_STAMP_REDEMPTION',
    order_id: orderId,
    stamp_activity_id: stampActivityId,
  });

  const response = await sendCommand(command);
  ensureSuccess(response, 'Cancel stamp redemption');
};
