import { useEffect, useState, useCallback } from "react";
import { BookOpen, Brain, CheckCircle2, FileText, Plus, Trash2, Upload, Users } from "lucide-react";
import { api, CampaignSnapshot, ModuleKnowledgeFile, ModuleRecord } from "../lib/api";

interface Props {
  campaignId: string;
  snapshot: CampaignSnapshot;
  onRefresh: () => Promise<void>;
  onError: (msg: string) => void;
}

interface DraftNpc {
  name: string;
  notes: string;
  skills_csv: string;
}

export function ModulesPanel({ campaignId, snapshot, onRefresh, onError }: Props) {
  const [modules, setModules] = useState<ModuleRecord[]>([]);
  const [showBuilder, setShowBuilder] = useState(false);
  const [loadingId, setLoadingId] = useState<string | null>(null);
  const [moveStart, setMoveStart] = useState(false);
  const [statusMsg, setStatusMsg] = useState<string | null>(null);

  // Builder state
  const [modName, setModName] = useState("");
  const [modDesc, setModDesc] = useState("");
  const [modSystem, setModSystem] = useState("");
  const [modBrief, setModBrief] = useState("");
  const [files, setFiles] = useState<ModuleKnowledgeFile[]>([]);
  const [pasteFileName, setPasteFileName] = useState("lore_notes.md");
  const [pasteContent, setPasteContent] = useState("");
  const [draftNpcs, setDraftNpcs] = useState<DraftNpc[]>([]);
  const [npcName, setNpcName] = useState("");
  const [npcSkills, setNpcSkills] = useState("Broker-2, Streetwise-1");
  const [npcNotes, setNpcNotes] = useState("");

  const refreshModules = useCallback(async () => {
    try {
      const res = await api.listModules();
      setModules(res.modules);
    } catch (e) {
      onError((e as Error).message);
    }
  }, [onError]);

  useEffect(() => {
    refreshModules();
  }, [refreshModules]);

  async function handleFileUpload(fileList: FileList | null) {
    if (!fileList || fileList.length === 0) return;
    for (const f of Array.from(fileList)) {
      const text = await f.text();
      if (f.name.toLowerCase().endsWith(".json")) {
        try {
          const parsed = JSON.parse(text);
          if (parsed.name && Array.isArray(parsed.knowledge_files)) {
            await api.createModule(parsed);
            setStatusMsg(`Imported module bundle '${parsed.name}'!`);
            await refreshModules();
            continue;
          }
        } catch {
          // treat as plain knowledge file if not a module bundle
        }
      }
      setFiles((prev) => [
        ...prev,
        {
          filename: f.name,
          title: f.name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " "),
          content: text,
        },
      ]);
    }
  }

  function handleAddPastedFile() {
    if (!pasteContent.trim()) return;
    setFiles((prev) => [
      ...prev,
      {
        filename: pasteFileName.trim() || `doc_${prev.length + 1}.md`,
        title: (pasteFileName.trim() || "Knowledge Doc").replace(/\.[^.]+$/, ""),
        content: pasteContent.trim(),
      },
    ]);
    setPasteContent("");
  }

  function handleAddNpc() {
    if (!npcName.trim()) return;
    setDraftNpcs((prev) => [
      ...prev,
      { name: npcName.trim(), skills_csv: npcSkills.trim(), notes: npcNotes.trim() },
    ]);
    setNpcName("");
    setNpcNotes("");
  }

  async function handleSaveModule() {
    if (!modName.trim()) {
      onError("Enter a module name.");
      return;
    }
    const npcsPayload = draftNpcs.map((n) => {
      const skills: Record<string, number> = {};
      for (const part of n.skills_csv.split(",")) {
        const p = part.trim();
        if (!p) continue;
        const m = p.match(/^(.+?)[-\s](\d+)$/);
        if (m) skills[m[1].trim()] = Number(m[2]);
        else skills[p] = 1;
      }
      return {
        kind: "npc",
        name: n.name,
        species: "Human",
        age: 34,
        upp: { str_val: 7, dex_val: 7, end_val: 7, int_val: 8, edu_val: 8, soc_val: 7 },
        skills,
        augments: [],
        salary_cr: 0,
        notes: n.notes,
      };
    });

    try {
      await api.createModule({
        slug: "",
        name: modName.trim(),
        version: "1.0",
        description: modDesc.trim(),
        author: "Custom",
        starting_system_name: modSystem.trim() || null,
        referee_brief: modBrief.trim() || modDesc.trim(),
        knowledge_files: files,
        npcs: npcsPayload,
        ships: [],
        events: [],
        tags: ["custom"],
        source: "custom",
      });
      setModName("");
      setModDesc("");
      setModSystem("");
      setModBrief("");
      setFiles([]);
      setDraftNpcs([]);
      setShowBuilder(false);
      setStatusMsg("Custom module saved! Click 'Load into Game' to ingest into Hindsight.");
      await refreshModules();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  async function handleLoadModule(m: ModuleRecord) {
    setLoadingId(m.id);
    setStatusMsg(null);
    try {
      const res = await api.loadModule(campaignId, m.id, moveStart);
      setStatusMsg(
        `Loaded '${res.module_name}': ingested ${res.hindsight_chunks_ingested} knowledge chunk(s) into Hindsight (${res.hindsight_bank_id}), spawned ${res.npcs_spawned} NPC(s) and ${res.events_spawned} event(s).`
      );
      await onRefresh();
    } catch (e) {
      onError((e as Error).message);
    } finally {
      setLoadingId(null);
    }
  }

  async function handleDeleteModule(m: ModuleRecord) {
    try {
      await api.deleteModule(m.id);
      await refreshModules();
    } catch (e) {
      onError((e as Error).message);
    }
  }

  const loadedIds = new Set((snapshot.loaded_modules || []).map((lm) => lm.module_id));

  return (
    <div className="space-y-3 text-xs">
      <div className="flex items-center justify-between bg-slate-900 border border-slate-800 rounded p-2.5">
        <div>
          <div className="font-semibold text-purple-300 flex items-center gap-1.5">
            <BookOpen className="w-4 h-4" /> Adventure Modules & Hindsight Knowledge
          </div>
          <div className="text-[10px] text-slate-400">
            Load modules to ingest markdown/text lore into this game's Hindsight bank and spawn NPCs.
          </div>
        </div>
        <button
          onClick={() => setShowBuilder((s) => !s)}
          className="bg-purple-700 hover:bg-purple-600 text-white rounded px-2.5 py-1 flex items-center gap-1 shrink-0"
        >
          <Plus className="w-3.5 h-3.5" /> {showBuilder ? "Close" : "New Module"}
        </button>
      </div>

      {statusMsg && (
        <div className="p-2 rounded bg-emerald-950/60 border border-emerald-700 text-emerald-200 text-[11px]">
          {statusMsg}
        </div>
      )}

      {/* Loaded in Active Game */}
      {snapshot.loaded_modules && snapshot.loaded_modules.length > 0 && (
        <div className="bg-purple-950/30 border border-purple-800/60 rounded p-2.5 space-y-1.5">
          <div className="font-semibold text-purple-200 flex items-center gap-1">
            <Brain className="w-3.5 h-3.5" /> Active in This Game ({snapshot.loaded_modules.length})
          </div>
          {snapshot.loaded_modules.map((lm) => (
            <div
              key={lm.id}
              className="bg-slate-950/80 border border-purple-900/50 rounded p-2 flex items-center justify-between"
            >
              <div>
                <div className="font-semibold text-slate-100">{lm.module_name}</div>
                <div className="text-[10px] text-purple-300">
                  {lm.hindsight_chunks_ingested} Hindsight chunk(s) · {lm.npcs_spawned} NPC(s) ·{" "}
                  {lm.events_spawned} event(s)
                </div>
              </div>
              <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
            </div>
          ))}
        </div>
      )}

      {/* Custom Module Creator & File Uploader */}
      {showBuilder && (
        <div className="bg-slate-900 border border-purple-700/70 rounded p-3 space-y-2.5">
          <div className="font-semibold text-purple-300">Create or Upload Module Bundle</div>
          <div className="grid grid-cols-2 gap-1.5">
            <input
              className="bg-slate-950 border border-slate-700 rounded px-2 py-1"
              placeholder="Module Name (e.g. Pirates of Drinax)"
              value={modName}
              onChange={(e) => setModName(e.target.value)}
            />
            <input
              className="bg-slate-950 border border-slate-700 rounded px-2 py-1"
              placeholder="Starting System (optional, e.g. Drinax)"
              value={modSystem}
              onChange={(e) => setModSystem(e.target.value)}
            />
          </div>
          <input
            className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1"
            placeholder="Short description…"
            value={modDesc}
            onChange={(e) => setModDesc(e.target.value)}
          />
          <textarea
            rows={2}
            className="w-full bg-slate-950 border border-slate-700 rounded px-2 py-1"
            placeholder="Referee brief (injected into story ledger so the AI immediately knows the hook)…"
            value={modBrief}
            onChange={(e) => setModBrief(e.target.value)}
          />

          {/* Knowledge Files Upload */}
          <div className="border-t border-slate-800 pt-2 space-y-1.5">
            <div className="flex items-center justify-between">
              <span className="font-semibold text-slate-300 flex items-center gap-1">
                <FileText className="w-3.5 h-3.5 text-cyan-400" /> Knowledge Files ({files.length})
              </span>
              <label className="cursor-pointer bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded px-2 py-0.5 text-[11px] flex items-center gap-1">
                <Upload className="w-3 h-3" /> Upload .md/.txt/.json
                <input
                  type="file"
                  multiple
                  accept=".md,.txt,.json"
                  className="hidden"
                  onChange={(e) => handleFileUpload(e.target.files)}
                />
              </label>
            </div>
            {files.map((f, idx) => (
              <div
                key={idx}
                className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded px-2 py-1 text-[11px]"
              >
                <span className="truncate">
                  {f.filename} ({f.content.length.toLocaleString()} chars)
                </span>
                <button
                  onClick={() => setFiles((p) => p.filter((_, i) => i !== idx))}
                  className="text-red-400 hover:text-red-300"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
            <div className="flex gap-1">
              <input
                className="w-36 bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
                value={pasteFileName}
                onChange={(e) => setPasteFileName(e.target.value)}
                placeholder="filename.md"
              />
              <input
                className="flex-1 bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
                value={pasteContent}
                onChange={(e) => setPasteContent(e.target.value)}
                placeholder="Or paste knowledge/lore text here…"
              />
              <button
                onClick={handleAddPastedFile}
                className="bg-slate-800 hover:bg-slate-700 rounded px-2 text-[11px]"
              >
                + Doc
              </button>
            </div>
          </div>

          {/* Module NPCs */}
          <div className="border-t border-slate-800 pt-2 space-y-1.5">
            <div className="font-semibold text-slate-300 flex items-center gap-1">
              <Users className="w-3.5 h-3.5 text-amber-400" /> Included NPCs ({draftNpcs.length})
            </div>
            {draftNpcs.map((n, idx) => (
              <div
                key={idx}
                className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded px-2 py-1 text-[11px]"
              >
                <span className="truncate">
                  <strong className="text-slate-200">{n.name}</strong> ({n.skills_csv})
                </span>
                <button
                  onClick={() => setDraftNpcs((p) => p.filter((_, i) => i !== idx))}
                  className="text-red-400"
                >
                  <Trash2 className="w-3 h-3" />
                </button>
              </div>
            ))}
            <div className="grid grid-cols-2 gap-1">
              <input
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
                placeholder="NPC Name"
                value={npcName}
                onChange={(e) => setNpcName(e.target.value)}
              />
              <input
                className="bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
                placeholder="Skills (e.g. Broker-2, Gun Combat-1)"
                value={npcSkills}
                onChange={(e) => setNpcSkills(e.target.value)}
              />
            </div>
            <div className="flex gap-1">
              <input
                className="flex-1 bg-slate-950 border border-slate-700 rounded px-1.5 py-1 text-[11px]"
                placeholder="NPC bio / motivation / location…"
                value={npcNotes}
                onChange={(e) => setNpcNotes(e.target.value)}
              />
              <button
                onClick={handleAddNpc}
                className="bg-slate-800 hover:bg-slate-700 rounded px-2 text-[11px]"
              >
                + NPC
              </button>
            </div>
          </div>

          <button
            onClick={handleSaveModule}
            className="w-full bg-purple-600 hover:bg-purple-500 text-white font-semibold rounded py-1.5"
          >
            Save Module to Library
          </button>
        </div>
      )}

      {/* Available Modules Catalog */}
      <div className="space-y-2">
        <div className="flex items-center justify-between text-[11px] text-slate-400">
          <span className="font-semibold text-slate-300">Available Modules ({modules.length})</span>
          <label className="flex items-center gap-1">
            <input
              type="checkbox"
              checked={moveStart}
              onChange={(e) => setMoveStart(e.target.checked)}
            />
            Move ship to module starting system
          </label>
        </div>

        {modules.map((m) => {
          const isLoaded = loadedIds.has(m.id);
          return (
            <div key={m.id} className="bg-slate-900 border border-slate-800 rounded p-2.5 space-y-1.5">
              <div className="flex items-start justify-between gap-2">
                <div>
                  <div className="font-bold text-slate-100 text-sm">
                    {m.name}{" "}
                    <span className="text-[10px] font-mono text-slate-400">v{m.version}</span>
                  </div>
                  <div className="text-[11px] text-slate-400">{m.description}</div>
                </div>
                {m.source === "custom" && (
                  <button
                    onClick={() => handleDeleteModule(m)}
                    className="text-slate-500 hover:text-red-400"
                    title="Delete custom module"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>

              <div className="flex flex-wrap gap-1.5 text-[10px]">
                <span className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-cyan-300">
                  {m.knowledge_files.length} knowledge file(s)
                </span>
                <span className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-amber-300">
                  {m.npcs.length} NPC(s)
                </span>
                <span className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-emerald-300">
                  {m.events.length} event(s)
                </span>
                {m.starting_system_name && (
                  <span className="px-1.5 py-0.5 rounded bg-slate-950 border border-slate-800 text-slate-300">
                    Start: {m.starting_system_name}
                  </span>
                )}
              </div>

              {m.knowledge_files.length > 0 && (
                <div className="text-[10px] text-slate-400">
                  Docs: {m.knowledge_files.map((k) => k.filename).join(", ")}
                </div>
              )}

              <button
                disabled={loadingId === m.id}
                onClick={() => handleLoadModule(m)}
                className={`w-full py-1.5 rounded font-semibold flex items-center justify-center gap-1.5 ${
                  isLoaded
                    ? "bg-slate-800 hover:bg-slate-700 text-purple-200 border border-purple-700/60"
                    : "bg-purple-700 hover:bg-purple-600 text-white"
                }`}
              >
                <Brain className="w-3.5 h-3.5" />
                {loadingId === m.id
                  ? "Ingesting into Hindsight…"
                  : isLoaded
                  ? "Re-Ingest Knowledge into Hindsight"
                  : "Load Module into Game (Hindsight + NPCs)"}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
