<script lang="ts">
  import { socketClient } from '$lib/socket/client';
  import * as AlertDialog from '$lib/components/ui/alert-dialog';
  import { Button } from '$lib/components/ui/button';
  import { Progress } from '$lib/components/ui/progress';
  import { Spinner } from '$lib/components/ui/spinner';

  const state = socketClient.state;

  let progress = $derived(
    ($state.reconnectAttempt / $state.maxReconnectAttempts) * 100
  );

  function handleCancel() {
    socketClient.disconnect();
  }
</script>

<AlertDialog.Root open={$state.status === 'reconnecting'}>
  <AlertDialog.Portal>
    <AlertDialog.Overlay />
    <AlertDialog.Content class="sm:max-w-sm">
      <AlertDialog.Header class="text-center sm:text-center">
        <div class="flex justify-center mb-4">
          <Spinner class="size-10 text-primary" />
        </div>
        <AlertDialog.Title>Reconnecting...</AlertDialog.Title>
        <AlertDialog.Description>
          Attempting to restore your connection
        </AlertDialog.Description>
      </AlertDialog.Header>

      <div class="space-y-4 py-4">
        <div class="text-sm text-muted-foreground text-center">
          Attempt {$state.reconnectAttempt} of {$state.maxReconnectAttempts}
        </div>
        <Progress value={progress} class="h-1.5" />
      </div>

      <AlertDialog.Footer class="sm:justify-center">
        <Button variant="secondary" onclick={handleCancel}>
          Cancel
        </Button>
      </AlertDialog.Footer>
    </AlertDialog.Content>
  </AlertDialog.Portal>
</AlertDialog.Root>
