<script lang="ts">
    import * as Resizable from "$lib/components/ui/resizable";
    import { Button, buttonVariants } from "$lib/components/ui/button";
    import { ScrollArea } from "$lib/components/ui/scroll-area";
    import * as Table from "$lib/components/ui/table";
    import { Input } from "$lib/components/ui/input";
    import { Badge } from "$lib/components/ui/badge";
    import { Separator } from "$lib/components/ui/separator";
    import { Label } from "$lib/components/ui/label";
    import * as Tabs from "$lib/components/ui/tabs";
    import * as Card from "$lib/components/ui/card";
    import * as DropdownMenu from "$lib/components/ui/dropdown-menu";
    import * as Dialog from "$lib/components/ui/dialog";
    import {
        Plus,
        Search,
        Settings,
        MoreVertical,
        Copy,
        Eye,
        EyeOff,
        Trash2,
        Monitor,
        Box,
        CheckCircle2,
    } from "@lucide/svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    // Types
    type Variable = { key: string; value: string; visible: boolean };
    type Profile = { name: string; color?: string; vars: Variable[] };
    type App = {
        id: string;
        name: string;
        description: string;
        profiles: Profile[];
        activeProfile?: string;
    };

    // State
    let apps: App[] = $state([]);
    let selectedAppId = $state("");
    let selectedProfileIndex = $state(0);
    let searchTerm = $state("");
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    let loading = $state(true);
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    let error = $state("");

    // Dialog State
    let showNewAppDialog = $state(false);
    let newAppName = $state("");
    let newAppDesc = $state("");
    let showNewProfileDialog = $state(false);
    let newProfileName = $state("");
    let copyFromProfile = $state("");

    // ... onMount ...

    // ... saveToBackend ...

    // New Functions
    // Variable Actions
    let showAddVarDialog = $state(false);
    let newVarKey = $state("");
    let newVarValue = $state("");

    function addVariable() {
        if (!newVarKey.trim() || !selectedApp) return;

        const app = apps.find((a) => a.id === selectedAppId);
        if (!app) return;
        const profile = app.profiles[selectedProfileIndex];

        // Check duplicate key
        if (profile.vars.some((v) => v.key === newVarKey.trim())) {
            return;
        }

        profile.vars = [
            ...profile.vars,
            {
                key: newVarKey.trim(),
                value: newVarValue,
                visible: false, // Default to hidden for secrets
            },
        ];

        newVarKey = "";
        newVarValue = "";
        showAddVarDialog = false;
        saveToBackend();
    }

    function deleteVariable(key: string) {
        const app = apps.find((a) => a.id === selectedAppId);
        if (!app) return;
        const profile = app.profiles[selectedProfileIndex];
        if (!profile) return;

        profile.vars = profile.vars.filter((v) => v.key !== key);
        saveToBackend();
    }

    function createApp() {
        if (!newAppName.trim()) return;

        const newApp: App = {
            id: newAppName.trim(), // Use name as ID
            name: newAppName.trim(),
            description: newAppDesc.trim(),
            profiles: [{ name: "default", vars: [] }],
            activeProfile: "default",
        };

        apps = [...apps, newApp];
        selectedAppId = newApp.id;
        selectedProfileIndex = 0;

        newAppName = "";
        newAppDesc = "";
        showNewAppDialog = false;

        saveToBackend();
    }

    function createProfile() {
        if (!newProfileName.trim() || !selectedApp) return;

        // Check duplicate
        if (selectedApp.profiles.some((p) => p.name === newProfileName.trim()))
            return;

        let initialVars: Variable[] = [];
        if (copyFromProfile) {
            const source = selectedApp.profiles.find(
                (p) => p.name === copyFromProfile,
            );
            if (source) {
                // Deep copy
                initialVars = source.vars.map((v) => ({ ...v }));
            }
        }

        const newProfile: Profile = {
            name: newProfileName.trim(),
            vars: initialVars,
        };

        // Reset copy selection
        copyFromProfile = "";

        const appIndex = apps.findIndex((a) => a.id === selectedAppId);
        if (appIndex !== -1) {
            apps[appIndex].profiles = [...apps[appIndex].profiles, newProfile];
            // Trigger reactivity is handled by $state proxy but sometimes array mutation needs care
            // apps = apps;
            selectedProfileIndex = apps[appIndex].profiles.length - 1;
        }

        newProfileName = "";
        showNewProfileDialog = false;

        saveToBackend();
    }

    // ... rest of script

    // Backend Response Types
    type BackendState = {
        apps: Record<string, BackendApp>;
    };
    type BackendApp = {
        target_binary: string;
        active_profile?: string;
        profiles: Record<string, Record<string, string>>;
    };

    onMount(async () => {
        try {
            const config = await invoke<BackendState>("get_config");
            console.log("Loaded config:", config);

            // Transform Backend Data to Frontend Model
            const loadedApps: App[] = Object.entries(config.apps).map(
                ([name, appData]) => {
                    const profiles = Object.entries(appData.profiles).map(
                        ([pName, vars]) => {
                            return {
                                name: pName,
                                vars: Object.entries(vars).map(([k, v]) => ({
                                    key: k,
                                    value: v,
                                    visible: false,
                                })),
                            };
                        },
                    );

                    return {
                        id: name, // Using name as ID for now
                        name: name,
                        description: appData.target_binary,
                        profiles: profiles,
                        activeProfile: appData.active_profile,
                    };
                },
            );

            apps = loadedApps;
            if (apps.length > 0) {
                selectedAppId = apps[0].id;
            }
        } catch (err) {
            console.error("Failed to load config:", err);
            error = String(err);
        } finally {
            loading = false;
        }
    });

    async function saveToBackend() {
        if (!selectedApp) return;

        // Reconstruct State from Frontend Store
        const appsMap: Record<string, BackendApp> = {};

        apps.forEach((app) => {
            const profiles: Record<string, Record<string, string>> = {};
            app.profiles.forEach((p) => {
                const vars: Record<string, string> = {};
                p.vars.forEach((v) => {
                    vars[v.key] = v.value;
                });
                profiles[p.name] = vars;
            });

            appsMap[app.id] = {
                target_binary: app.description,
                active_profile: app.activeProfile,
                profiles: profiles,
            };
        });

        const statePayload = {
            apps: appsMap,
            extra: {},
        };

        try {
            await invoke("save_config", { state: statePayload });
            console.log("Saved config successfully");
        } catch (err) {
            console.error("Failed to save config:", err);
        }
    }

    function setActiveProfile(appName: string, profileName: string) {
        const app = apps.find((a) => a.id === appName);
        if (app) {
            app.activeProfile = profileName;
            // Force re-assignment to trigger reactivity if needed,
            // though $state array mutation should be fine in Svelte 5 with proxies.
            // But let's be safe.
        }
        saveToBackend();
    }

    let selectedApp = $derived(apps.find((a) => a.id === selectedAppId));
    let currentProfile = $derived(selectedApp?.profiles[selectedProfileIndex]);

    function toggleVisibility(v: any) {
        v.visible = !v.visible;
    }
</script>

<Resizable.PaneGroup
    direction="horizontal"
    class="h-screen w-full bg-background text-foreground"
>
    <!-- Sidebar -->
    <Resizable.Pane
        defaultSize={20}
        minSize={15}
        maxSize={30}
        class="border-r bg-background flex flex-col overflow-hidden"
    >
        <div class="p-4 border-b flex items-center gap-2">
            <Box class="h-6 w-6 text-primary" />
            <h1 class="font-bold text-xl tracking-tight">EnvHub</h1>
        </div>

        <div class="p-4">
            <Dialog.Root bind:open={showNewAppDialog}>
                <Dialog.Trigger>
                    <Button
                        variant="outline"
                        class="w-full justify-start gap-2"
                    >
                        <Plus class="h-4 w-4" />
                        New Application
                    </Button>
                </Dialog.Trigger>
                <Dialog.Content>
                    <Dialog.Header>
                        <Dialog.Title>Create New Application</Dialog.Title>
                        <Dialog.Description>
                            Add a new application to manage its environment
                            variables.
                        </Dialog.Description>
                    </Dialog.Header>
                    <div class="grid gap-4 py-4">
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label for="name" class="text-right">Name</Label>
                            <Input
                                id="name"
                                bind:value={newAppName}
                                class="col-span-3"
                                placeholder="e.g. My Service"
                            />
                        </div>
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label for="desc" class="text-right">Command</Label>
                            <Input
                                id="desc"
                                bind:value={newAppDesc}
                                class="col-span-3"
                                placeholder="e.g. python, node, or /path/to/bin"
                            />
                        </div>
                    </div>
                    <Dialog.Footer>
                        <Button type="submit" onclick={createApp}
                            >Create Application</Button
                        >
                    </Dialog.Footer>
                </Dialog.Content>
            </Dialog.Root>
        </div>

        <ScrollArea class="flex-1 px-2">
            <div class="space-y-1">
                {#each apps as app}
                    <button
                        class="w-full text-left px-3 py-2 rounded-md transition-colors flex items-center gap-3 {selectedAppId ===
                        app.id
                            ? 'bg-primary/10 text-primary font-medium'
                            : 'hover:bg-muted text-muted-foreground'}"
                        onclick={() => {
                            selectedAppId = app.id;
                            selectedProfileIndex = 0;
                        }}
                    >
                        <Monitor class="h-4 w-4" />
                        <span class="truncate">{app.name}</span>
                    </button>
                {/each}
            </div>
        </ScrollArea>

        <div class="p-4 border-t">
            <Button
                variant="ghost"
                class="w-full justify-start gap-2 text-muted-foreground"
            >
                <Settings class="h-4 w-4" />
                Settings
            </Button>
        </div>
    </Resizable.Pane>

    <Resizable.Handle />

    <!-- Main Content -->
    <Resizable.Pane defaultSize={80}>
        <div class="flex flex-col h-full overflow-hidden">
            {#if selectedApp}
                <!-- App Header Check -->
                <div
                    class="border-b px-6 py-4 flex items-center justify-between bg-card/50 backdrop-blur-sm"
                >
                    <div>
                        <h2 class="text-2xl font-bold">{selectedApp.name}</h2>
                        <p class="text-muted-foreground text-sm">
                            {selectedApp.description ||
                                "No description provided."}
                        </p>
                    </div>
                    <div class="flex items-center gap-2">
                        <DropdownMenu.Root>
                            <DropdownMenu.Trigger
                                class={buttonVariants({
                                    variant: "ghost",
                                    size: "icon",
                                })}
                            >
                                <MoreVertical class="h-4 w-4" />
                            </DropdownMenu.Trigger>
                            <DropdownMenu.Content align="end">
                                <DropdownMenu.Item>Sync</DropdownMenu.Item>
                                <DropdownMenu.Item>Export</DropdownMenu.Item>
                                <DropdownMenu.Separator />
                                <DropdownMenu.Item class="text-destructive"
                                    >Delete App</DropdownMenu.Item
                                >
                            </DropdownMenu.Content>
                        </DropdownMenu.Root>
                    </div>
                </div>
                <div class="px-6 pt-4 border-b bg-muted/5">
                    <Tabs.Root value={currentProfile?.name} class="w-full">
                        <Tabs.List
                            class="w-full justify-start bg-transparent p-0 border-b-0 gap-6"
                        >
                            {#each selectedApp.profiles as profile, i}
                                <div class="relative group">
                                    <button
                                        class="pb-3 px-1 border-b-2 transition-all text-sm font-medium flex items-center gap-2 {selectedProfileIndex ===
                                        i
                                            ? 'border-primary text-primary'
                                            : 'border-transparent text-muted-foreground hover:text-foreground'}"
                                        onclick={() =>
                                            (selectedProfileIndex = i)}
                                    >
                                        {profile.name}
                                        {#if selectedApp.activeProfile === profile.name}
                                            <Badge
                                                variant="default"
                                                class="h-5 px-1.5 text-[10px]"
                                                >Active</Badge
                                            >
                                        {/if}
                                    </button>
                                    {#if selectedApp.activeProfile !== profile.name}
                                        <button
                                            class="absolute -top-2 -right-2 hidden group-hover:block bg-primary text-primary-foreground rounded-full p-0.5 shadow-sm"
                                            title="Set as Active"
                                            onclick={(e) => {
                                                e.stopPropagation();
                                                setActiveProfile(
                                                    selectedApp.name,
                                                    profile.name,
                                                );
                                            }}
                                        >
                                            <CheckCircle2 class="h-3 w-3" />
                                        </button>
                                    {/if}
                                </div>
                            {/each}
                            <Dialog.Root bind:open={showNewProfileDialog}>
                                <Dialog.Trigger>
                                    <button
                                        class="pb-3 px-1 text-sm font-medium text-muted-foreground hover:text-primary transition-colors flex items-center gap-1"
                                    >
                                        <Plus class="h-3 w-3" /> New Profile
                                    </button>
                                </Dialog.Trigger>
                                <Dialog.Content>
                                    <Dialog.Header>
                                        <Dialog.Title
                                            >Create New Profile</Dialog.Title
                                        >
                                        <Dialog.Description>
                                            Add a new profile (e.g. staging,
                                            production) to this application.
                                        </Dialog.Description>
                                    </Dialog.Header>
                                    <div class="grid gap-4 py-4">
                                        <div
                                            class="grid grid-cols-4 items-center gap-4"
                                        >
                                            <Label
                                                for="p-name"
                                                class="text-right">Name</Label
                                            >
                                            <Input
                                                id="p-name"
                                                bind:value={newProfileName}
                                                class="col-span-3"
                                                placeholder="e.g. Production"
                                            />
                                        </div>
                                        <div
                                            class="grid grid-cols-4 items-center gap-4"
                                        >
                                            <Label
                                                for="p-copy"
                                                class="text-right"
                                                >Copy From</Label
                                            >
                                            <select
                                                id="p-copy"
                                                bind:value={copyFromProfile}
                                                class="col-span-3 flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                                            >
                                                <option value=""
                                                    >(None - Empty Profile)</option
                                                >
                                                {#each selectedApp?.profiles || [] as profile}
                                                    <option value={profile.name}
                                                        >{profile.name}</option
                                                    >
                                                {/each}
                                            </select>
                                        </div>
                                    </div>
                                    <Dialog.Footer>
                                        <Button
                                            type="submit"
                                            onclick={createProfile}
                                            >Create Profile</Button
                                        >
                                    </Dialog.Footer>
                                </Dialog.Content>
                            </Dialog.Root>
                        </Tabs.List>
                    </Tabs.Root>
                </div>

                <!-- Toolbar -->
                <div class="p-6 flex items-center justify-between gap-4">
                    <div class="relative flex-1 max-w-md">
                        <Search
                            class="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground"
                        />
                        <Input
                            type="search"
                            placeholder="Search variables..."
                            class="pl-9"
                            bind:value={searchTerm}
                        />
                    </div>
                    <Dialog.Root bind:open={showAddVarDialog}>
                        <Dialog.Trigger>
                            <Button>
                                <Plus class="h-4 w-4 mr-2" />
                                Add Variable
                            </Button>
                        </Dialog.Trigger>
                        <Dialog.Content>
                            <Dialog.Header>
                                <Dialog.Title>Add Variable</Dialog.Title>
                                <Dialog.Description>
                                    Create a new environment variable.
                                </Dialog.Description>
                            </Dialog.Header>
                            <div class="grid gap-4 py-4">
                                <div
                                    class="grid grid-cols-4 items-center gap-4"
                                >
                                    <Label for="v-key" class="text-right"
                                        >Key</Label
                                    >
                                    <Input
                                        id="v-key"
                                        bind:value={newVarKey}
                                        class="col-span-3"
                                        placeholder="e.g. DATABASE_URL"
                                    />
                                </div>
                                <div
                                    class="grid grid-cols-4 items-center gap-4"
                                >
                                    <Label for="v-val" class="text-right"
                                        >Value</Label
                                    >
                                    <Input
                                        id="v-val"
                                        bind:value={newVarValue}
                                        class="col-span-3"
                                        placeholder="Value"
                                    />
                                </div>
                            </div>
                            <Dialog.Footer>
                                <Button type="submit" onclick={addVariable}
                                    >Save Variable</Button
                                >
                            </Dialog.Footer>
                        </Dialog.Content>
                    </Dialog.Root>
                </div>

                <!-- Content -->
                <ScrollArea class="flex-1">
                    <div class="px-6 pb-6">
                        <div class="rounded-md border bg-card">
                            <Table.Root>
                                <Table.Header>
                                    <Table.Row>
                                        <Table.Head class="w-[300px]"
                                            >Key</Table.Head
                                        >
                                        <Table.Head>Value</Table.Head>
                                        <Table.Head class="w-[100px] text-right"
                                            >Actions</Table.Head
                                        >
                                    </Table.Row>
                                </Table.Header>
                                <Table.Body>
                                    {#each currentProfile?.vars.filter( (v) => v.key
                                                .toLowerCase()
                                                .includes(searchTerm.toLowerCase()), ) || [] as variable}
                                        <Table.Row>
                                            <Table.Cell
                                                class="font-mono font-medium"
                                                >{variable.key}</Table.Cell
                                            >
                                            <Table.Cell>
                                                <div
                                                    class="flex items-center gap-2 group"
                                                >
                                                    <div
                                                        class="flex-1 max-w-[400px]"
                                                    >
                                                        <Input
                                                            type={variable.visible
                                                                ? "text"
                                                                : "password"}
                                                            class="font-mono text-sm h-8"
                                                            bind:value={
                                                                variable.value
                                                            }
                                                            onblur={() =>
                                                                saveToBackend()}
                                                        />
                                                    </div>
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        class="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                                                        onclick={() =>
                                                            toggleVisibility(
                                                                variable,
                                                            )}
                                                    >
                                                        {#if variable.visible}
                                                            <EyeOff
                                                                class="h-3 w-3"
                                                            />
                                                        {:else}
                                                            <Eye
                                                                class="h-3 w-3"
                                                            />
                                                        {/if}
                                                    </Button>
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        class="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                                                        title="Copy"
                                                    >
                                                        <Copy class="h-3 w-3" />
                                                    </Button>
                                                </div>
                                            </Table.Cell>
                                            <Table.Cell class="text-right">
                                                <Button
                                                    variant="ghost"
                                                    size="icon"
                                                    class="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                    onclick={() =>
                                                        deleteVariable(
                                                            variable.key,
                                                        )}
                                                >
                                                    <Trash2 class="h-4 w-4" />
                                                </Button>
                                            </Table.Cell>
                                        </Table.Row>
                                    {/each}
                                </Table.Body>
                            </Table.Root>
                        </div>
                    </div>
                </ScrollArea>
            {:else}
                <div
                    class="flex-1 flex items-center justify-center text-muted-foreground"
                >
                    Select an application to view details
                </div>
            {/if}
        </div>
    </Resizable.Pane>
</Resizable.PaneGroup>
