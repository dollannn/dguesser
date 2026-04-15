export function load({ params }: { params: { id: string } }) {
  return { partyId: params.id };
}
