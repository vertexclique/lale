import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface BoardConfig {
  name: string;
  isa: string;
  core: string;
  pipeline_stages: number;
  instruction_cache?: {
    size_kb: number;
    line_size_bytes: number;
    associativity: number;
  };
  data_cache?: {
    size_kb: number;
    line_size_bytes: number;
    associativity: number;
  };
  soc?: {
    name: string;
    cpu_frequency_mhz: number;
    memory_regions: number;
  };
  board?: string;
}

export default function Configuration() {
  const [boards, setBoards] = useState<string[]>([]);
  const [selectedBoard, setSelectedBoard] = useState<string>('');
  const [boardDetails, setBoardDetails] = useState<BoardConfig | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState<string>('');

  // Load boards on mount
  useEffect(() => {
    loadBoards();
  }, []);

  // Load selected board from localStorage
  useEffect(() => {
    const saved = localStorage.getItem('selectedBoard');
    if (saved && boards.includes(saved)) {
      setSelectedBoard(saved);
      loadBoardDetails(saved);
    } else if (boards.length > 0) {
      // Default to first platform config or cortex-m4
      const defaultBoard = boards.find(b => b.startsWith('platforms/')) || 
                          boards.find(b => b.includes('cortex-m4')) ||
                          boards[0];
      setSelectedBoard(defaultBoard);
      loadBoardDetails(defaultBoard);
    }
  }, [boards]);

  const loadBoards = async () => {
    try {
      setLoading(true);
      const result = await invoke<string[]>('list_board_configs');
      setBoards(result);
      setError(null);
    } catch (err) {
      setError(`Failed to load boards: ${err}`);
    } finally {
      setLoading(false);
    }
  };

  const loadBoardDetails = async (boardName: string) => {
    if (!boardName) return;
    
    try {
      setLoading(true);
      const details = await invoke<BoardConfig>('validate_board_config', {
        boardName,
      });
      setBoardDetails(details);
      setError(null);
    } catch (err) {
      setError(`Failed to load board details: ${err}`);
      setBoardDetails(null);
    } finally {
      setLoading(false);
    }
  };

  const handleBoardSelect = (boardName: string) => {
    setSelectedBoard(boardName);
    localStorage.setItem('selectedBoard', boardName);
    loadBoardDetails(boardName);
  };

  // Filter platforms by search query
  const platforms = boards
    .filter(b => b.startsWith('platforms/'))
    .filter(b => {
      if (!searchQuery) return true;
      const boardName = b.replace('platforms/', '').toLowerCase();
      return boardName.includes(searchQuery.toLowerCase());
    });

  return (
    <div className="p-6">
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Board Configuration
        </h1>
        <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">
          Select a board configuration for WCET analysis. This configuration will be used for all new analyses.
        </p>
      </div>

      {error && (
        <div className="mb-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <p className="text-sm text-red-800 dark:text-red-200">{error}</p>
        </div>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Board Selection */}
        <div className="lg:col-span-1">
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
            <div className="p-4 border-b border-gray-200 dark:border-gray-700">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                Available Boards
              </h2>
            </div>
            
            <div className="p-4 space-y-4">
              {/* Search Input */}
              {boards.length > 0 && (
                <div className="relative">
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder="Search boards..."
                    className="w-full px-3 py-2 pl-9 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 text-sm focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                  />
                  <svg
                    className="absolute left-3 top-2.5 h-4 w-4 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                    />
                  </svg>
                </div>
              )}

              {loading && boards.length === 0 ? (
                <div className="text-center py-8">
                  <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mx-auto"></div>
                  <p className="mt-2 text-sm text-gray-600 dark:text-gray-400">Loading boards...</p>
                </div>
              ) : (
                <>
                  {platforms.length > 0 ? (
                    <div>
                      <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Platform Configurations
                      </h3>
                      <div className="space-y-1">
                        {platforms.map((board) => (
                          <button
                            key={board}
                            onClick={() => handleBoardSelect(board)}
                            className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors ${
                              selectedBoard === board
                                ? 'bg-blue-50 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 font-medium'
                                : 'text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700'
                            }`}
                          >
                            {board.replace('platforms/', '')}
                          </button>
                        ))}
                      </div>
                    </div>
                  ) : (
                    <div className="text-center py-8">
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        No platform configurations found
                      </p>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        </div>

        {/* Board Details */}
        <div className="lg:col-span-2">
          <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
            <div className="p-4 border-b border-gray-200 dark:border-gray-700">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                Configuration Details
              </h2>
            </div>

            <div className="p-6">
              {loading && boardDetails === null ? (
                <div className="text-center py-12">
                  <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto"></div>
                  <p className="mt-4 text-sm text-gray-600 dark:text-gray-400">Loading configuration...</p>
                </div>
              ) : boardDetails ? (
                <div className="space-y-6">
                  {/* Selected Board Badge */}
                  <div className="flex items-center gap-2">
                    <span className="px-3 py-1 bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300 text-sm font-medium rounded-full">
                      ✓ Active Configuration
                    </span>
                  </div>

                  {/* Core Information */}
                  <div>
                    <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                      Core Information
                    </h3>
                    <dl className="grid grid-cols-2 gap-4">
                      <div>
                        <dt className="text-xs text-gray-500 dark:text-gray-400">ISA</dt>
                        <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                          {boardDetails.isa}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-xs text-gray-500 dark:text-gray-400">Core</dt>
                        <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                          {boardDetails.core}
                        </dd>
                      </div>
                      <div>
                        <dt className="text-xs text-gray-500 dark:text-gray-400">Pipeline Stages</dt>
                        <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                          {boardDetails.pipeline_stages}
                        </dd>
                      </div>
                      {boardDetails.board && (
                        <div>
                          <dt className="text-xs text-gray-500 dark:text-gray-400">Board</dt>
                          <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                            {boardDetails.board}
                          </dd>
                        </div>
                      )}
                    </dl>
                  </div>

                  {/* Cache Configuration */}
                  {(boardDetails.instruction_cache || boardDetails.data_cache) && (
                    <div>
                      <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                        Cache Configuration
                      </h3>
                      <div className="space-y-4">
                        {boardDetails.instruction_cache && (
                          <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                            <h4 className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">
                              Instruction Cache
                            </h4>
                            <dl className="grid grid-cols-3 gap-4">
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Size</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.instruction_cache.size_kb} KB
                                </dd>
                              </div>
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Line Size</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.instruction_cache.line_size_bytes} B
                                </dd>
                              </div>
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Associativity</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.instruction_cache.associativity}-way
                                </dd>
                              </div>
                            </dl>
                          </div>
                        )}

                        {boardDetails.data_cache && (
                          <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                            <h4 className="text-xs font-medium text-gray-700 dark:text-gray-300 mb-2">
                              Data Cache
                            </h4>
                            <dl className="grid grid-cols-3 gap-4">
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Size</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.data_cache.size_kb} KB
                                </dd>
                              </div>
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Line Size</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.data_cache.line_size_bytes} B
                                </dd>
                              </div>
                              <div>
                                <dt className="text-xs text-gray-500 dark:text-gray-400">Associativity</dt>
                                <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                  {boardDetails.data_cache.associativity}-way
                                </dd>
                              </div>
                            </dl>
                          </div>
                        )}
                      </div>
                    </div>
                  )}

                  {/* SoC Information */}
                  {boardDetails.soc && (
                    <div>
                      <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-3">
                        SoC Information
                      </h3>
                      <dl className="grid grid-cols-3 gap-4">
                        <div>
                          <dt className="text-xs text-gray-500 dark:text-gray-400">SoC</dt>
                          <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                            {boardDetails.soc.name}
                          </dd>
                        </div>
                        <div>
                          <dt className="text-xs text-gray-500 dark:text-gray-400">CPU Frequency</dt>
                          <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                            {boardDetails.soc.cpu_frequency_mhz} MHz
                          </dd>
                        </div>
                        <div>
                          <dt className="text-xs text-gray-500 dark:text-gray-400">Memory Regions</dt>
                          <dd className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                            {boardDetails.soc.memory_regions}
                          </dd>
                        </div>
                      </dl>
                    </div>
                  )}

                  {/* Info Box */}
                  <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                    <p className="text-sm text-blue-800 dark:text-blue-200">
                      <strong>Note:</strong> This configuration will be used for all new WCET analyses. 
                      You can change it at any time from this page.
                    </p>
                  </div>
                </div>
              ) : (
                <div className="text-center py-12">
                  <p className="text-gray-500 dark:text-gray-400">
                    Select a board configuration to view details
                  </p>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
