import {
  PanelSection,
  PanelSectionRow,
  staticClasses,
  Dropdown,
  SliderField,
  ToggleField,
  ButtonItem,
  TextField,
  showModal,
  ConfirmModal
} from "@decky/ui";
import {
  callable,
  toaster,
  definePlugin
} from "@decky/api"

import { useState, useEffect, useMemo } from "react";
import { FaGamepad } from "react-icons/fa";

const getGlobalConfig = callable<[], any>("get_config");
const saveGlobalConfig = callable<[config: any], boolean>("save_config");
const getSlotsData = callable<[], any>("get_slots_data");
const saveSlotsData = callable<[data: any], boolean>("save_slots_data");
const getDaemonState = callable<[], boolean>("get_daemon_state");
const setDaemonState = callable<[is_running: boolean], boolean>("set_daemon_state");

// Module-level flag read by the notification poller, which runs on the plugin
// instance (outside <Content/>) and therefore has no access to the config state.
// Error toasts are always shown; info toasts honour this flag. <Content/> keeps
// it in sync with the `show_info_toasts` global config value.
let showInfoToasts = true;

const btnOptions = [
  { data: 317, label: "L3 (Left Stick Click)" },
  { data: 318, label: "R3 (Right Stick Click)" },
  { data: 310, label: "L1 (Left Bumper)" },
  { data: 311, label: "R1 (Right Bumper)" },
  { data: 314, label: "Select Button" },
  { data: 315, label: "Start Button" },
];

const SlotEditModal = ({ slotId, initialData, slotsData, updateSlots, closeModal }: any) => {
  const [name, setName] = useState(initialData?.name || "");
  const [loops, setLoops] = useState(initialData?.loops || 0);
  const [cancel, setCancel] = useState(initialData?.cancel_on_input ?? true);

  return (
    <ConfirmModal 
      onCancel={closeModal} 
      onOK={() => {
        const newData = { ...slotsData };
        newData.slots[slotId] = { name: name || "Unnamed Slot", loops, cancel_on_input: cancel };
        newData.active_slot = slotId;
        updateSlots(newData);
        closeModal();
      }}
      strTitle="Configure Slot"
    >
      <div style={{ padding: "0 0 20px 0" }}>
        <TextField 
          label="Slot Name"
          value={name} 
          onChange={(e: any) => {
            if (e.target.value.length <= 30) {
              setName(e.target.value);
            }
          }} 
        />
        <SliderField 
          label="Loop count" 
          description="How many times the replay should run (0 is infinite)" 
          value={loops} 
          min={0} 
          max={50} 
          step={1} 
          showValue={true} 
          onChange={setLoops} 
        />
        <ToggleField 
          label="Cancel on input" 
          description="Cancel replay if a button is pressed"
          checked={cancel} 
          onChange={setCancel} 
        />
      </div>
    </ConfirmModal>
  );
};

const SettingsModal = ({ initialConfig, onSave, closeModal }: any) => {
  const [recordCode, setRecordCode] = useState(initialConfig?.trigger_record_code ?? 317);
  const [replayCode, setReplayCode] = useState(initialConfig?.trigger_replay_code ?? 318);
  const [holdMs, setHoldMs] = useState(initialConfig?.hold_duration_ms ?? 500);

  return (
    <ConfirmModal
      onCancel={closeModal}
      onOK={() => {
        onSave({
          ...initialConfig,
          trigger_record_code: recordCode,
          trigger_replay_code: replayCode,
          hold_duration_ms: holdMs,
        });
        closeModal();
      }}
      strTitle="Daemon Settings"
    >
      <div style={{ padding: "0 0 20px 0" }}>
        <div style={{ fontSize: "11px", color: "#bbb", marginBottom: "12px", lineHeight: "1.3" }}>
          These settings are read when the daemon starts. Stop the daemon before changing them.
        </div>

        <div style={{ marginBottom: "5px" }}><b>Record Trigger:</b></div>
        <Dropdown
          rgOptions={btnOptions}
          selectedOption={recordCode}
          onChange={(opt: any) => setRecordCode(opt.data)}
        />

        <div style={{ marginTop: "12px", marginBottom: "5px" }}><b>Replay Trigger:</b></div>
        <Dropdown
          rgOptions={btnOptions}
          selectedOption={replayCode}
          onChange={(opt: any) => setReplayCode(opt.data)}
        />

        <div style={{ marginTop: "12px" }}>
          <SliderField
            label="Trigger hold time"
            description="How long to hold Record/Replay before it activates (0 = trigger on press)"
            value={holdMs / 1000}
            min={0}
            max={2}
            step={0.1}
            showValue={true}
            valueSuffix="s"
            onChange={(v: number) => setHoldMs(Math.round(v * 1000))}
          />
        </div>
      </div>
    </ConfirmModal>
  );
};

function Content() {
  const [globalConfig, setGlobalConfig] = useState<any>(null);
  const [slotsData, setSlotsData] = useState<any>(null);
  const [isDaemonRunning, setIsDaemonRunning] = useState<boolean>(false);

  useEffect(() => {
    getGlobalConfig().then(setGlobalConfig);
    getSlotsData().then(setSlotsData);
    getDaemonState().then(setIsDaemonRunning);
  }, []);

  // Keep the module-level toast flag (read by the notification poller) in sync.
  useEffect(() => {
    showInfoToasts = globalConfig ? (globalConfig.show_info_toasts ?? true) : true;
  }, [globalConfig]);

  const setGlobal = async (newConfig: any) => {
    setGlobalConfig(newConfig);
    await saveGlobalConfig(newConfig);
  };

  const updateSlots = async (newData: any) => {
    setSlotsData(newData);
    await saveSlotsData(newData);
  };

  const toggleDaemon = async (state: boolean) => {
    // Optimistically set the UI toggle
    setIsDaemonRunning(state);
    try {
      await setDaemonState(state);
    } catch (e) {
      console.error("Daemon failed to start!", e);
      // Revert the toggle if the backend rejected the start command
      getDaemonState().then(setIsDaemonRunning);
    }
  };

  const handleNew = () => {
    if (!slotsData) return;
    const ids = Object.keys(slotsData.slots).map(Number);
    const newId = (ids.length > 0 ? Math.max(...ids) + 1 : 0).toString();
    showModal(
      <SlotEditModal 
        slotId={newId} 
        initialData={{ name: `Slot ${newId}`, loops: 0, cancel_on_input: true }}
        slotsData={slotsData} 
        updateSlots={updateSlots} 
      />
    );
  };

  const handleEdit = () => {
    if (!slotsData) return;
    const activeId = slotsData.active_slot;
    showModal(
      <SlotEditModal 
        slotId={activeId} 
        initialData={slotsData.slots[activeId]}
        slotsData={slotsData} 
        updateSlots={updateSlots} 
      />
    );
  };

  const handleDelete = () => {
    if (!slotsData) return;
    const activeId = slotsData.active_slot;
    if (activeId === "0") return; // Cannot delete default slot

    showModal(
      <ConfirmModal 
        strTitle="Delete Slot?" 
        strDescription="Are you sure you want to delete this macro slot?"
        onOK={() => {
          const newData = { ...slotsData };
          delete newData.slots[activeId];
          newData.active_slot = "0"; // Default back to slot 0
          updateSlots(newData);
        }}
      />
    );
  };

  const openSettings = () => {
    if (!globalConfig) return;
    showModal(
      <SettingsModal
        initialConfig={globalConfig}
        onSave={setGlobal}
      />
    );
  };

  // Memoize options so Dropdown doesn't lose focus / reset scroll position
  const slotOptions = useMemo(() => {
    if (!slotsData) return [];
    return Object.entries(slotsData.slots).map(([id, s]: [string, any]) => {
      return {
        data: id,
        label: (
          <div style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
            {s.name}
          </div>
        )
      };
    });
  }, [slotsData]);

  if (!globalConfig || !slotsData) return <div>Loading...</div>;

  const activeSlotId = slotsData.active_slot;

  return (
    <>
      <div style={{ padding: "10px", fontSize: "12px", lineHeight: "1.4", color: "#bbb" }}>
        Decky-Macro listens (via a daemon) on hardware inputs (/dev/input/event*) to trigger/record inputs, and injects them into the same hardware device. This plugin is not aware if the inputs are injected into a game or steam client.
      </div>

      <PanelSection>
        <div style={{ padding: "0 10px 10px 10px", fontSize: "12px", color: "#ffcc00", fontWeight: "bold" }}>
          NOTE: currently only supports digital buttons, not analog ones (trigger/sticks).
        </div>
        <PanelSectionRow>
          <ToggleField
            label="Enable Decky Macro"
            checked={isDaemonRunning}
            onChange={(val: boolean) => toggleDaemon(val)}
          />
        </PanelSectionRow>
        <PanelSectionRow>
          <div style={{ padding: 10, backgroundColor: "rgba(0,0,0,0.3)", borderRadius: 4, fontSize: 13, lineHeight: "1.5" }}>
            <b>Status: </b> 
            {isDaemonRunning ? (
              <span style={{ color: "#00ffcc" }}>Running</span>
            ) : (
              <span style={{ color: "#ffcc00" }}>Stopped</span>
            )}
          </div>
        </PanelSectionRow>
      </PanelSection>

      <PanelSection>
        <PanelSectionRow>
          <ButtonItem
            layout="below"
            onClick={openSettings}
            disabled={isDaemonRunning}
          >
            Settings
          </ButtonItem>
        </PanelSectionRow>
        {isDaemonRunning && (
          <div style={{ padding: "5px 10px 0 10px", fontSize: "11px", color: "#ffcc00", lineHeight: "1.3" }}>
            Stop the daemon to change trigger settings.
          </div>
        )}
      </PanelSection>

      <PanelSection title="Active Macro Slot">
        <PanelSectionRow>
          <Dropdown
            rgOptions={slotOptions}
            selectedOption={activeSlotId}
            onChange={(opt: any) => {
              setTimeout(() => updateSlots({ ...slotsData, active_slot: opt.data }), 0);
            }}
          />
        </PanelSectionRow>

        <PanelSectionRow>
          <div style={{ display: "flex", flexDirection: "column", gap: "5px", marginTop: "10px" }}>
            <ButtonItem layout="below" onClick={handleNew}>
              New Slot
            </ButtonItem>
            <ButtonItem layout="below" onClick={handleEdit}>
              Edit Selected Slot
            </ButtonItem>
            <ButtonItem 
              layout="below" 
              onClick={handleDelete}
              disabled={activeSlotId === "0"}
            >
              Delete Selected Slot
            </ButtonItem>
          </div>
        </PanelSectionRow>
      </PanelSection>

      <PanelSection title="Notifications">
        <PanelSectionRow>
          <ToggleField
            label="Show info toasts"
            description="Show status toasts (recording started/saved, playback started/finished). Error toasts are always shown."
            checked={globalConfig.show_info_toasts ?? true}
            onChange={(val: boolean) => {
              setTimeout(() => setGlobal({ ...globalConfig, show_info_toasts: val }), 0);
            }}
          />
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}


const getNotifications = callable<[], { message: string; error?: boolean }[]>("get_notifications");

export default definePlugin(() => {
  const interval = setInterval(async () => {
    try {
      const res = await getNotifications();
      if (res && res.length > 0) {
        for (const n of res) {
          const isError = !!n?.error;
          // Error toasts are always shown; info toasts honour the setting.
          if (!isError && !showInfoToasts) continue;
          toaster.toast({
            title: isError ? "Decky Macro (error)" : "Decky Macro",
            body: n?.message ?? String(n),
            duration: isError ? 4000 : 1500,
            sound: 0,
          });
        }
      }
    } catch (e) {
      // ignore
    }
  }, 1000);

  return {
    name: "Decky Macro",
    titleView: <div className={staticClasses.Title}>Decky Macro</div>,
    content: <Content />,
    icon: <FaGamepad />,
    onDismount() {
      clearInterval(interval);
    },
  };
});
