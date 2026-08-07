import { useEffect, useMemo, useState, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useParams } from 'react-router-dom'
import { api, type PlayerCard } from '../api/client'
import { LANGUAGE_OPTIONS, type SupportedLanguage } from '../i18n'

const NON_OF = 'Non of the above'

function TroopRow({
  title,
  levels,
  tgs,
  bandLabel,
  giftLabel,
  value,
  onLevel,
  onTg,
}: {
  title: string
  levels: readonly { v: string; label: string }[]
  tgs: readonly { v: string; label: string }[]
  bandLabel: string
  giftLabel: string
  value: { level_band: string; tg_band: string }
  onLevel: (v: string) => void
  onTg: (v: string) => void
}) {
  return (
    <div className="border border-gray-600 rounded-xl p-4 space-y-3">
      <h3 className="text-lg font-semibold text-teal-300">{title}</h3>
      <div className="grid sm:grid-cols-2 gap-4">
        <div>
          <label className="block text-xs text-gray-400 mb-1">{bandLabel}</label>
          <select
            className="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white text-sm"
            value={value.level_band}
            onChange={(e) => onLevel(e.target.value)}
          >
            {levels.map((o) => (
              <option key={o.v} value={o.v}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
        <div>
          <label className="block text-xs text-gray-400 mb-1">{giftLabel}</label>
          <select
            className="w-full px-3 py-2 rounded-lg bg-gray-900 border border-gray-600 text-white text-sm"
            value={value.tg_band}
            onChange={(e) => onTg(e.target.value)}
          >
            {tgs.map((o) => (
              <option key={o.v} value={o.v}>
                {o.label}
              </option>
            ))}
          </select>
        </div>
      </div>
    </div>
  )
}

export default function TyrantFormPage() {
  const { code } = useParams<{ code: string }>()
  const { t, i18n } = useTranslation()
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [config, setConfig] = useState<Record<string, unknown> | null>(null)

  const [playerId, setPlayerId] = useState('')
  const [lookupLoading, setLookupLoading] = useState(false)
  const [lookupErr, setLookupErr] = useState('')
  const [playerCard, setPlayerCard] = useState<PlayerCard | null>(null)
  const [isNameEditable, setIsNameEditable] = useState(false)

  const [alliance, setAlliance] = useState('')
  const [customAlliance, setCustomAlliance] = useState('')

  const [archer, setArcher] = useState({ level_band: 'level_1_9', tg_band: 'below_tg5' })
  const [cavalry, setCavalry] = useState({ level_band: 'level_1_9', tg_band: 'below_tg5' })
  const [infantry, setInfantry] = useState({ level_band: 'level_1_9', tg_band: 'below_tg5' })

  const [participateFullFiveHours, setParticipateFullFiveHours] = useState(false)
  /** Empty until user selects yes/no (required to submit). */
  const [autoHelpMonthCard, setAutoHelpMonthCard] = useState<'yes' | 'no' | ''>('')

  const [submitting, setSubmitting] = useState(false)
  const [submitOk, setSubmitOk] = useState(false)
  const [submitErr, setSubmitErr] = useState('')

  const changeLang = (l: SupportedLanguage) => {
    i18n.changeLanguage(l)
  }

  const levelOpts = useMemo(
    () =>
      [
        { v: 'level_1_9', label: t('tyrantLevel1to9') },
        { v: 'level_10', label: t('tyrantLevel10') },
        { v: 'level_11', label: t('tyrantLevel11') },
      ] as const,
    [t, i18n.language]
  )

  const tgOpts = useMemo(
    () =>
      [
        { v: 'below_tg5', label: t('tyrantTgBelowTg5') },
        { v: 'tg5', label: t('tyrantTg5') },
        { v: 'tg6', label: t('tyrantTg6') },
        { v: 'tg7', label: t('tyrantTg7') },
        { v: 'tg8', label: t('tyrantTg8') },
      ] as const,
    [t, i18n.language]
  )

  useEffect(() => {
    if (playerCard && playerId.trim() !== String(playerCard.player_id ?? '')) {
      setPlayerCard(null)
    }
  }, [playerId, playerCard])

  useEffect(() => {
    if (!code) return
    setLoading(true)
    setError(null)
    api.getTyrantFormConfig(code).then(({ ok, data, error: err }) => {
      if (ok && data) {
        const d = data as { config?: Record<string, unknown> }
        setConfig(d.config ?? {})
      } else setError(err ?? t('formNotFound'))
      setLoading(false)
    })
  }, [code, t])

  const alliances = useMemo(() => {
    const raw = (config?.alliances as string[] | undefined) ?? []
    const list = [...raw]
    const non = config?.include_non_of_above !== false
    if (non && !list.includes(NON_OF)) list.push(NON_OF)
    return list
  }, [config])

  const effectiveAlliance = alliance === NON_OF ? customAlliance.trim() : alliance

  async function lookupPlayer() {
    const id = playerId.trim()
    if (!id || !/^[0-9]+$/.test(id) || !code) {
      setLookupErr(t('playerIdMustBeNumber'))
      return
    }
    setLookupLoading(true)
    setLookupErr('')
    setPlayerCard(null)
    setIsNameEditable(false)
    const { ok, data } = await api.tyrantPlayerLookup(code, id)
    setLookupLoading(false)
    const d = data as PlayerCard & {
      success?: boolean
      kingdom_mismatch?: boolean
      error?: string
      name?: string
      is_fallback?: boolean
    }
    if (ok && d?.success && d?.name) {
      setPlayerCard({
        player_id: d.player_id ?? id,
        name: d.name,
        castle_level: d.castle_level != null ? String(d.castle_level) : undefined,
        kingdom: d.kingdom,
        avatar_image: d.avatar_image,
      })
      if (d?.is_fallback) {
        setIsNameEditable(true)
      }
    } else {
      const errMsg = d?.error as string | undefined
      setLookupErr(
        d?.kingdom_mismatch
          ? t('playerNotInKingdom')
          : errMsg && errMsg.length > 0
            ? errMsg
            : t('tyrantLookupFailed')
      )
    }
  }

  async function submit(ev: FormEvent) {
    ev.preventDefault()
    if (!code) return
    const id = playerId.trim()
    if (!id || !/^[0-9]+$/.test(id)) {
      setSubmitErr(t('playerIdMustBeNumber'))
      return
    }
    if (!effectiveAlliance) {
      setSubmitErr(alliance === NON_OF ? t('pleaseEnterCustomAlliance') : t('pleaseSelectAlliance'))
      return
    }
    if (!playerCard?.name?.trim()) {
      setSubmitErr(t('tyrantConfirmPlayerLookup'))
      return
    }
    if (autoHelpMonthCard !== 'yes' && autoHelpMonthCard !== 'no') {
      setSubmitErr(t('tyrantSelectAutoHelpMonthCard'))
      return
    }
    setSubmitting(true)
    setSubmitErr('')
    const { ok, error: err } = await api.tyrantSubmit(code, {
      player_id: id,
      player_name: playerCard.name.trim(),
      alliance: effectiveAlliance,
      archer,
      cavalry,
      infantry,
      utc_slots: [],
      participate_full_five_hours: participateFullFiveHours,
      auto_help_month_card_active: autoHelpMonthCard === 'yes',
    })
    setSubmitting(false)
    if (ok) {
      setSubmitOk(true)
    } else {
      setSubmitErr(err ?? t('failedToSubmitForm'))
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-900 flex items-center justify-center">
        <p className="text-xl text-gray-400">
          <i className="fas fa-spinner fa-spin mr-3"></i>
          {t('loadingFormConfiguration')}
        </p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="min-h-screen bg-gray-900 flex items-center justify-center p-8">
        <div className="bg-red-900/40 border border-red-700 text-red-200 px-8 py-6 rounded-xl max-w-md text-center">{error}</div>
      </div>
    )
  }

  const bandL = t('troopLevel')
  const giftL = t('troopTruegoldLevel')

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-950 via-teal-950/30 to-gray-900 py-12 px-4">
      <div className="max-w-2xl mx-auto">
        <div className="flex justify-end mb-4">
          <select
            value={i18n.language}
            onChange={(e) => changeLang(e.target.value as SupportedLanguage)}
            className="px-4 py-2 bg-gray-800 border border-gray-600 rounded-lg text-white focus:border-teal-500 focus:ring-2 focus:ring-teal-500/50 outline-none transition-all"
          >
            {LANGUAGE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </div>

        <div className="text-center mb-10">
          <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-teal-500/20 mb-4">
            <i className="fas fa-dragon text-4xl text-teal-400"></i>
          </div>
          <h1 className="text-3xl font-bold text-white">{t('tyrantPageTitle')}</h1>
          <p className="text-gray-400 mt-2">{t('tyrantPageSubtitle')}</p>
        </div>

        {submitOk ? (
          <div className="bg-gray-800/90 border border-teal-500/40 rounded-2xl p-10 text-center">
            <i className="fas fa-check-circle text-5xl text-teal-400 mb-4"></i>
            <p className="text-xl text-white font-medium">{t('tyrantSuccessMessage')}</p>
          </div>
        ) : (
          <form onSubmit={submit} className="space-y-8 bg-gray-800/80 border border-gray-700 rounded-2xl p-8 shadow-xl">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">{t('playerIdQuestion')}</label>
              <p className="text-xs text-gray-500 mb-2">{t('playerIdNote')}</p>
              <div className="flex gap-2 flex-wrap">
                <input
                  className="flex-1 min-w-[200px] px-4 py-3 rounded-xl bg-gray-900 border border-gray-600 text-white font-mono"
                  value={playerId}
                  onChange={(e) => setPlayerId(e.target.value.replace(/\D/g, ''))}
                  placeholder={t('playerIdPlaceholder')}
                  autoComplete="off"
                />
                <button
                  type="button"
                  onClick={() => void lookupPlayer()}
                  disabled={lookupLoading}
                  className="px-4 py-3 rounded-xl bg-teal-600 hover:bg-teal-500 text-white font-medium disabled:opacity-50"
                >
                  {lookupLoading ? t('lookingUp') : t('confirm')}
                </button>
              </div>
              {lookupErr && <p className="text-red-400 text-sm mt-2">{lookupErr}</p>}
              {playerCard && (
                <div className="mt-4 flex items-center gap-4 p-4 rounded-xl bg-gray-900/80 border border-gray-600">
                  {playerCard.avatar_image ? (
                    <img src={playerCard.avatar_image} alt="" className="w-14 h-14 rounded-full object-cover" />
                  ) : (
                    <div className="w-14 h-14 rounded-full bg-teal-600 flex items-center justify-center text-xl text-white">
                      {(playerCard.name || '?').charAt(0).toUpperCase()}
                    </div>
                  )}
                  <div className="flex-1 min-w-0">
                    {isNameEditable ? (
                      <div className="space-y-1">
                        <label className="block text-xs font-semibold text-teal-400 uppercase tracking-wide">
                          {t('characterNameQuestion')}
                        </label>
                        <input
                          type="text"
                          value={playerCard.name}
                          onChange={(e) => {
                            const val = e.target.value
                            setPlayerCard((p) => p ? { ...p, name: val } : null)
                          }}
                          required
                          className="w-full max-w-xs px-3 py-1.5 rounded-lg bg-gray-800 border border-gray-600 text-white text-sm focus:border-teal-500 focus:ring-1 focus:ring-teal-500/50 outline-none transition-all"
                          placeholder="Enter your in-game name"
                        />
                      </div>
                    ) : (
                      <p className="text-white font-medium">{playerCard.name}</p>
                    )}
                    <p className="text-xs text-gray-400">
                      {playerCard.player_id}
                      {playerCard.castle_level ? ` · ${t('castleLevel')}: ${playerCard.castle_level}` : ''}
                      {playerCard.kingdom ? ` · ${t('kingdom')}: ${playerCard.kingdom}` : ''}
                    </p>
                  </div>
                </div>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">{t('allianceQuestion')}</label>
              {alliances.length > 0 ? (
                <>
                  <select
                    className="w-full px-4 py-3 rounded-xl bg-gray-900 border border-gray-600 text-white"
                    value={alliance}
                    onChange={(e) => setAlliance(e.target.value)}
                  >
                    <option value="">{t('tyrantSelectPlaceholder')}</option>
                    {alliances.map((a) => (
                      <option key={a} value={a}>
                        {a === NON_OF ? t('nonOfAbove') : a}
                      </option>
                    ))}
                  </select>
                  {alliance === NON_OF && (
                    <input
                      className="mt-3 w-full px-4 py-3 rounded-xl bg-gray-900 border border-gray-600 text-white"
                      placeholder={t('customAlliancePlaceholder')}
                      value={customAlliance}
                      onChange={(e) => setCustomAlliance(e.target.value)}
                    />
                  )}
                </>
              ) : (
                <input
                  className="w-full px-4 py-3 rounded-xl bg-gray-900 border border-gray-600 text-white"
                  value={alliance}
                  onChange={(e) => setAlliance(e.target.value)}
                  placeholder={t('tyrantAllianceFreePlaceholder')}
                />
              )}
            </div>

            <TroopRow
              title={t('tyrantArcher')}
              levels={levelOpts}
              tgs={tgOpts}
              bandLabel={bandL}
              giftLabel={giftL}
              value={archer}
              onLevel={(v) => setArcher((s) => ({ ...s, level_band: v }))}
              onTg={(v) => setArcher((s) => ({ ...s, tg_band: v }))}
            />
            <TroopRow
              title={t('tyrantCavalry')}
              levels={levelOpts}
              tgs={tgOpts}
              bandLabel={bandL}
              giftLabel={giftL}
              value={cavalry}
              onLevel={(v) => setCavalry((s) => ({ ...s, level_band: v }))}
              onTg={(v) => setCavalry((s) => ({ ...s, tg_band: v }))}
            />
            <TroopRow
              title={t('tyrantInfantry')}
              levels={levelOpts}
              tgs={tgOpts}
              bandLabel={bandL}
              giftLabel={giftL}
              value={infantry}
              onLevel={(v) => setInfantry((s) => ({ ...s, level_band: v }))}
              onTg={(v) => setInfantry((s) => ({ ...s, tg_band: v }))}
            />

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">{t('tyrantAutoHelpMonthCard')}</label>
              <p className="text-xs text-gray-500 mb-2">{t('tyrantAutoHelpMonthCardHint')}</p>
              <select
                className="w-full px-4 py-3 rounded-xl bg-gray-900 border border-gray-600 text-white"
                value={autoHelpMonthCard}
                onChange={(e) => setAutoHelpMonthCard(e.target.value as 'yes' | 'no' | '')}
              >
                <option value="">{t('tyrantSelectPlaceholder')}</option>
                <option value="yes">{t('tyrantAutoHelpYes')}</option>
                <option value="no">{t('tyrantAutoHelpNo')}</option>
              </select>
            </div>

            <label className="flex items-start gap-3 cursor-pointer text-gray-200">
              <input
                type="checkbox"
                checked={participateFullFiveHours}
                onChange={(e) => setParticipateFullFiveHours(e.target.checked)}
                className="mt-1 w-5 h-5 rounded border-gray-600 bg-gray-900 text-teal-600 focus:ring-teal-500"
              />
              <span className="text-sm">{t('tyrantParticipateFullFiveHours')}</span>
            </label>

            {submitErr && <div className="text-red-400 text-sm">{submitErr}</div>}

            <button
              type="submit"
              disabled={submitting}
              className="w-full py-4 rounded-xl bg-gradient-to-r from-teal-600 to-cyan-600 hover:from-teal-500 hover:to-cyan-500 text-white font-bold text-lg shadow-lg disabled:opacity-50"
            >
              {submitting ? t('submitting') : t('tyrantSubmit')}
            </button>
          </form>
        )}
      </div>
    </div>
  )
}
