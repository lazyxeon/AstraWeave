window.BENCHMARK_DATA = {
  "lastUpdate": 1792321136000,
  "repoUrl": "https://github.com/lazyxeon/AstraWeave",
  "entries": {
    "AstraWeave Performance Benchmarks": [
      {
        "commit": {
          "author": {
            "name": "Cursor Agent",
            "username": "cursoragent",
            "email": "cursoragent@cursor.com"
          },
          "committer": {
            "name": "Cursor Agent",
            "username": "cursoragent",
            "email": "cursoragent@cursor.com"
          },
          "id": "b6811ec4e0a902af8a005dce3d464e233113884b",
          "message": "fix(ci): comment PR benches and split render into three shards",
          "timestamp": "2026-10-18T10:02:11Z",
          "url": "https://github.com/lazyxeon/AstraWeave/commit/b6811ec4e0a902af8a005dce3d464e233113884b"
        },
        "date": 1792321136000,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "astraweave-net::batch_operations/batch_filter/10",
            "unit": "ns",
            "value": 22963.60200621248
          },
          {
            "name": "astraweave-net::batch_operations/batch_filter/100",
            "unit": "ns",
            "value": 254077.609766187
          },
          {
            "name": "astraweave-net::batch_operations/batch_filter/50",
            "unit": "ns",
            "value": 126210.45159593911
          },
          {
            "name": "astraweave-net::batch_operations/delta_chain/10",
            "unit": "ns",
            "value": 103489.91563620156
          },
          {
            "name": "astraweave-net::batch_operations/delta_chain/20",
            "unit": "ns",
            "value": 308897.76238639763
          },
          {
            "name": "astraweave-net::batch_operations/delta_chain/5",
            "unit": "ns",
            "value": 40301.52940525721
          },
          {
            "name": "astraweave-net::delta_compression/apply_delta/100",
            "unit": "ns",
            "value": 3320.8346487374633
          },
          {
            "name": "astraweave-net::delta_compression/apply_delta/1000",
            "unit": "ns",
            "value": 41977.18328786025
          },
          {
            "name": "astraweave-net::delta_compression/apply_delta/500",
            "unit": "ns",
            "value": 19854.680937433317
          },
          {
            "name": "astraweave-net::delta_compression/compression_ratio/100e_20%",
            "unit": "ns",
            "value": 9865.058068725391
          },
          {
            "name": "astraweave-net::delta_compression/compression_ratio/100e_5%",
            "unit": "ns",
            "value": 9022.43918996671
          },
          {
            "name": "astraweave-net::delta_compression/compression_ratio/100e_50%",
            "unit": "ns",
            "value": 11075.171928971713
          },
          {
            "name": "astraweave-net::delta_compression/diff_snapshots/1000e_10%",
            "unit": "ns",
            "value": 196798.02229547704
          },
          {
            "name": "astraweave-net::delta_compression/diff_snapshots/100e_10%",
            "unit": "ns",
            "value": 9339.529118065384
          },
          {
            "name": "astraweave-net::delta_compression/diff_snapshots/100e_50%",
            "unit": "ns",
            "value": 11146.792627908273
          },
          {
            "name": "astraweave-net::delta_compression/diff_snapshots/500e_10%",
            "unit": "ns",
            "value": 69011.33547623515
          },
          {
            "name": "astraweave-net::delta_compression/diff_snapshots/500e_50%",
            "unit": "ns",
            "value": 83346.41652535992
          },
          {
            "name": "astraweave-net::entity_operations/clone_entity",
            "unit": "ns",
            "value": 5.609353839536417
          },
          {
            "name": "astraweave-net::entity_operations/compare_entities",
            "unit": "ns",
            "value": 1.5576694186329052
          },
          {
            "name": "astraweave-net::entity_operations/create_entity",
            "unit": "ns",
            "value": 14.42469928955228
          },
          {
            "name": "astraweave-net::entity_operations/distance_squared",
            "unit": "ns",
            "value": 0.6221947958095171
          },
          {
            "name": "astraweave-net::hash_computation/hash_entities/100",
            "unit": "ns",
            "value": 1601.9276427288466
          },
          {
            "name": "astraweave-net::hash_computation/hash_entities/1000",
            "unit": "ns",
            "value": 15965.632697872848
          },
          {
            "name": "astraweave-net::hash_computation/hash_entities/50",
            "unit": "ns",
            "value": 812.5112957975045
          },
          {
            "name": "astraweave-net::hash_computation/hash_entities/500",
            "unit": "ns",
            "value": 7973.791081542331
          },
          {
            "name": "astraweave-net::interest_filtering/fov_interest/1000e_90deg",
            "unit": "ns",
            "value": 15426.448865578706
          },
          {
            "name": "astraweave-net::interest_filtering/fov_interest/100e_45deg",
            "unit": "ns",
            "value": 1251.2177301544461
          },
          {
            "name": "astraweave-net::interest_filtering/fov_interest/500e_60deg",
            "unit": "ns",
            "value": 6041.081198630397
          },
          {
            "name": "astraweave-net::interest_filtering/full_interest/100",
            "unit": "ns",
            "value": 2673.0819575746164
          },
          {
            "name": "astraweave-net::interest_filtering/full_interest/1000",
            "unit": "ns",
            "value": 24526.36432817512
          },
          {
            "name": "astraweave-net::interest_filtering/full_interest/500",
            "unit": "ns",
            "value": 12439.129638944569
          },
          {
            "name": "astraweave-net::interest_filtering/radius_interest/1000e_r30",
            "unit": "ns",
            "value": 9035.402581566956
          },
          {
            "name": "astraweave-net::interest_filtering/radius_interest/100e_r10",
            "unit": "ns",
            "value": 1111.1521906765804
          },
          {
            "name": "astraweave-net::interest_filtering/radius_interest/500e_r20",
            "unit": "ns",
            "value": 4683.320419002745
          },
          {
            "name": "astraweave-net::net_serialization/delta_to_json/100",
            "unit": "ns",
            "value": 4636.5655136701525
          },
          {
            "name": "astraweave-net::net_serialization/delta_to_json/50",
            "unit": "ns",
            "value": 2504.2566500265716
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_from_json/10",
            "unit": "ns",
            "value": 2164.5316288976755
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_from_json/100",
            "unit": "ns",
            "value": 18722.774999196554
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_from_json/50",
            "unit": "ns",
            "value": 9648.900157991073
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_to_json/10",
            "unit": "ns",
            "value": 1471.9201239225386
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_to_json/100",
            "unit": "ns",
            "value": 12809.925801181853
          },
          {
            "name": "astraweave-net::net_serialization/snapshot_to_json/50",
            "unit": "ns",
            "value": 6522.928009217829
          },
          {
            "name": "astraweave-net::snapshot_operations/clone_snapshot/100",
            "unit": "ns",
            "value": 88.18045563319565
          },
          {
            "name": "astraweave-net::snapshot_operations/clone_snapshot/50",
            "unit": "ns",
            "value": 60.247979611977335
          },
          {
            "name": "astraweave-net::snapshot_operations/clone_snapshot/500",
            "unit": "ns",
            "value": 231.6912751886479
          },
          {
            "name": "astraweave-net::snapshot_operations/create_snapshot/10",
            "unit": "ns",
            "value": 47.885753637193275
          },
          {
            "name": "astraweave-net::snapshot_operations/create_snapshot/100",
            "unit": "ns",
            "value": 381.20828593002415
          },
          {
            "name": "astraweave-net::snapshot_operations/create_snapshot/1000",
            "unit": "ns",
            "value": 3441.2721241807076
          },
          {
            "name": "astraweave-net::snapshot_operations/create_snapshot/50",
            "unit": "ns",
            "value": 222.0680405230488
          },
          {
            "name": "astraweave-net::snapshot_operations/create_snapshot/500",
            "unit": "ns",
            "value": 1714.1465238221842
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_frame_marking/frame_budget_tracking_5000",
            "unit": "ns",
            "value": 216794.7863541965
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_frame_marking/frame_marks_10000",
            "unit": "ns",
            "value": 0.9335115580319004
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_frame_marking/frame_marks_with_zones_5000",
            "unit": "ns",
            "value": 4126673.611942776
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_frame_marking/frame_time_calculation_1000",
            "unit": "ns",
            "value": 29994.94268906803
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_lock_profiling/contention_analysis_20000",
            "unit": "ns",
            "value": 989924.7407056361
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_lock_profiling/deadlock_detection_1000",
            "unit": "ns",
            "value": 294322.76713505597
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_lock_profiling/hold_time_calculation_10000",
            "unit": "ns",
            "value": 563054.00234774
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_lock_profiling/lock_event_recording_50000",
            "unit": "ns",
            "value": 3055171.585
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_memory_profiling/allocation_tracking_50000",
            "unit": "ns",
            "value": 5023421.686999999
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_memory_profiling/category_tracking_20000",
            "unit": "ns",
            "value": 401506.04753182636
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_memory_profiling/fragmentation_analysis_5000",
            "unit": "ns",
            "value": 13211.963928892877
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_memory_profiling/leak_detection_10000",
            "unit": "ns",
            "value": 193389.92252869077
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_message_logging/formatted_messages_10000",
            "unit": "ns",
            "value": 2144917.267916667
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_message_logging/message_creation_50000",
            "unit": "ns",
            "value": 3791543.624285715
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_message_logging/message_filtering_20000",
            "unit": "ns",
            "value": 31535.197510413873
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_message_logging/message_ring_buffer_100000",
            "unit": "ns",
            "value": 3924514.969230769
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_plot_data/multiple_plots_10_x_5000",
            "unit": "ns",
            "value": 787416.0189100542
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_plot_data/plot_downsampling_100000",
            "unit": "ns",
            "value": 281541.73481237894
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_plot_data/plot_recording_50000",
            "unit": "ns",
            "value": 659185.1696186197
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_plot_data/plot_statistics_10000",
            "unit": "ns",
            "value": 262304.0824193135
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_zone_operations/nested_zones_depth_20",
            "unit": "ns",
            "value": 3518205.224476813
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_zone_operations/push_pop_cycle_50000",
            "unit": "ns",
            "value": 8472190.746666664
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_zone_operations/zone_creation_100000",
            "unit": "ns",
            "value": 2839666.7999999993
          },
          {
            "name": "astraweave-profiling::profiling_adversarial_zone_operations/zone_data_collection_10000",
            "unit": "ns",
            "value": 808040.0307142858
          },
          {
            "name": "astraweave-water::query_fleet_per_tick/1",
            "unit": "ns",
            "value": 753.9562274418141
          },
          {
            "name": "astraweave-water::query_fleet_per_tick/16",
            "unit": "ns",
            "value": 4835.271152504673
          },
          {
            "name": "astraweave-water::query_fleet_per_tick/256",
            "unit": "ns",
            "value": 56787.19773171181
          },
          {
            "name": "astraweave-water::query_fleet_per_tick/64",
            "unit": "ns",
            "value": 16175.725846825839
          },
          {
            "name": "astraweave-water::query_one_body_8_volumes",
            "unit": "ns",
            "value": 10.917698503360784
          },
          {
            "name": "astraweave-localsim::localsim_tick_32_busy",
            "unit": "ns",
            "value": 7817978.441428573
          },
          {
            "name": "astraweave-localsim::localsim_tick_32_regions",
            "unit": "ns",
            "value": 1800377.4926823396
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/batch_creation/100",
            "unit": "ns",
            "value": 141.38114782694163
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/batch_creation/20",
            "unit": "ns",
            "value": 21.43548619349313
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/batch_creation/5",
            "unit": "ns",
            "value": 12.137123982593215
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/batch_creation/50",
            "unit": "ns",
            "value": 98.88937461048698
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/clone",
            "unit": "ns",
            "value": 0.9411238292994922
          },
          {
            "name": "astraweave-cinematics::Camera Keyframes/single_creation",
            "unit": "ns",
            "value": 0.9495765810302248
          },
          {
            "name": "astraweave-cinematics::Edge Cases/exact_event_time",
            "unit": "ns",
            "value": 37.440661424372706
          },
          {
            "name": "astraweave-cinematics::Edge Cases/large_step",
            "unit": "ns",
            "value": 5305.379333638091
          },
          {
            "name": "astraweave-cinematics::Edge Cases/out_of_range_error",
            "unit": "ns",
            "value": 89.16358788314803
          },
          {
            "name": "astraweave-cinematics::Edge Cases/repeated_seeks",
            "unit": "ns",
            "value": 0.31739185566633815
          },
          {
            "name": "astraweave-cinematics::Edge Cases/sub_ms_step",
            "unit": "ns",
            "value": 85.47632256213997
          },
          {
            "name": "astraweave-cinematics::Edge Cases/zero_duration_step",
            "unit": "ns",
            "value": 12.218977991425673
          },
          {
            "name": "astraweave-cinematics::Event Creation/anim_start_event",
            "unit": "ns",
            "value": 9.26459258491052
          },
          {
            "name": "astraweave-cinematics::Event Creation/audio_play_event",
            "unit": "ns",
            "value": 9.9392029491126
          },
          {
            "name": "astraweave-cinematics::Event Creation/camera_key_event",
            "unit": "ns",
            "value": 6.900489592969842
          },
          {
            "name": "astraweave-cinematics::Event Creation/event_clone_complex",
            "unit": "ns",
            "value": 156.06348476393921
          },
          {
            "name": "astraweave-cinematics::Event Creation/fx_trigger_event",
            "unit": "ns",
            "value": 56.36791549280702
          },
          {
            "name": "astraweave-cinematics::Event Emission/animation_event",
            "unit": "ns",
            "value": 35.48040657502985
          },
          {
            "name": "astraweave-cinematics::Event Emission/audio_event",
            "unit": "ns",
            "value": 34.6956390695559
          },
          {
            "name": "astraweave-cinematics::Event Emission/camera_event",
            "unit": "ns",
            "value": 30.19733074983091
          },
          {
            "name": "astraweave-cinematics::Event Emission/fx_event",
            "unit": "ns",
            "value": 63.0208972679834
          },
          {
            "name": "astraweave-cinematics::Event Emission/multiple_events_step",
            "unit": "ns",
            "value": 376.2855266997191
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/deserialize_complex",
            "unit": "ns",
            "value": 6663.427831124252
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/deserialize_simple",
            "unit": "ns",
            "value": 1100.5132115953984
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/roundtrip_complex",
            "unit": "ns",
            "value": 14335.902812489054
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/serialize_complex",
            "unit": "ns",
            "value": 7101.212847183356
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/serialize_complex_pretty",
            "unit": "ns",
            "value": 14350.67585423046
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/serialize_empty",
            "unit": "ns",
            "value": 76.638111834624
          },
          {
            "name": "astraweave-cinematics::JSON Serialization/serialize_simple",
            "unit": "ns",
            "value": 1296.9225725833949
          },
          {
            "name": "astraweave-cinematics::Sequencer Operations/creation",
            "unit": "ns",
            "value": 0.6618752641858378
          },
          {
            "name": "astraweave-cinematics::Sequencer Operations/default",
            "unit": "ns",
            "value": 0.6591312807529411
          },
          {
            "name": "astraweave-cinematics::Sequencer Operations/seek",
            "unit": "ns",
            "value": 0.7033312522206836
          },
          {
            "name": "astraweave-cinematics::Sequencer Operations/step_empty_timeline",
            "unit": "ns",
            "value": 12.96014230095555
          },
          {
            "name": "astraweave-cinematics::Sequencer Operations/step_no_events",
            "unit": "ns",
            "value": 12.732535535712648
          },
          {
            "name": "astraweave-cinematics::Timeline Clone/100_tracks",
            "unit": "ns",
            "value": 6242.760179808681
          },
          {
            "name": "astraweave-cinematics::Timeline Clone/complex",
            "unit": "ns",
            "value": 564.1103496491934
          },
          {
            "name": "astraweave-cinematics::Timeline Clone/empty",
            "unit": "ns",
            "value": 23.12313847109957
          },
          {
            "name": "astraweave-cinematics::Timeline Clone/simple",
            "unit": "ns",
            "value": 46.147245490457955
          },
          {
            "name": "astraweave-cinematics::Timeline Creation/empty",
            "unit": "ns",
            "value": 10.384002147889216
          },
          {
            "name": "astraweave-cinematics::Timeline Creation/time_struct",
            "unit": "ns",
            "value": 0.6662649039476635
          },
          {
            "name": "astraweave-cinematics::Timeline Creation/with_long_name",
            "unit": "ns",
            "value": 12.193409699811921
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/60fps_1sec/100",
            "unit": "ns",
            "value": 20954.489296085223
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/60fps_1sec/20",
            "unit": "ns",
            "value": 3555.863996110928
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/60fps_1sec/5",
            "unit": "ns",
            "value": 743.4518634823509
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/60fps_1sec/50",
            "unit": "ns",
            "value": 9450.516226317492
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/complex_30sec_full",
            "unit": "ns",
            "value": 82886.87711813925
          },
          {
            "name": "astraweave-cinematics::Timeline Playback/complex_single_frame",
            "unit": "ns",
            "value": 49.85961875057533
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/camera_keyframes_step/10",
            "unit": "ns",
            "value": 118.82120113875543
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/camera_keyframes_step/100",
            "unit": "ns",
            "value": 596.3453624604876
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/camera_keyframes_step/50",
            "unit": "ns",
            "value": 350.9222086640631
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/camera_keyframes_step/500",
            "unit": "ns",
            "value": 2509.9349530594427
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/creation/10",
            "unit": "ns",
            "value": 439.4558844834205
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/creation/100",
            "unit": "ns",
            "value": 7943.9689432232035
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/creation/1000",
            "unit": "ns",
            "value": 87092.57048347528
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/creation/50",
            "unit": "ns",
            "value": 3688.4482027593767
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/creation/500",
            "unit": "ns",
            "value": 43714.43577936251
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/step/10",
            "unit": "ns",
            "value": 28.719364769558798
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/step/100",
            "unit": "ns",
            "value": 336.50173804616804
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/step/50",
            "unit": "ns",
            "value": 152.99601340143744
          },
          {
            "name": "astraweave-cinematics::Timeline Scaling/step/500",
            "unit": "ns",
            "value": 1741.1984841194694
          },
          {
            "name": "astraweave-cinematics::Track Creation/animation",
            "unit": "ns",
            "value": 9.412929718270885
          },
          {
            "name": "astraweave-cinematics::Track Creation/audio",
            "unit": "ns",
            "value": 9.094479018519772
          },
          {
            "name": "astraweave-cinematics::Track Creation/camera_5_keyframes",
            "unit": "ns",
            "value": 9.868244053301192
          },
          {
            "name": "astraweave-cinematics::Track Creation/fx_complex_params",
            "unit": "ns",
            "value": 676.1908169896801
          },
          {
            "name": "astraweave-cinematics::Track Creation/fx_simple_params",
            "unit": "ns",
            "value": 55.67027466322318
          },
          {
            "name": "astraweave-quests::batch_operations/check_completion_batch/10",
            "unit": "ns",
            "value": 44.84599255396165
          },
          {
            "name": "astraweave-quests::batch_operations/check_completion_batch/100",
            "unit": "ns",
            "value": 312.62197940527716
          },
          {
            "name": "astraweave-quests::batch_operations/check_completion_batch/50",
            "unit": "ns",
            "value": 164.77171848317815
          },
          {
            "name": "astraweave-quests::batch_operations/check_completion_batch/500",
            "unit": "ns",
            "value": 1712.936487691549
          },
          {
            "name": "astraweave-quests::batch_operations/validate_batch/10",
            "unit": "ns",
            "value": 6.730937023457805
          },
          {
            "name": "astraweave-quests::batch_operations/validate_batch/100",
            "unit": "ns",
            "value": 61.32630819136764
          },
          {
            "name": "astraweave-quests::batch_operations/validate_batch/50",
            "unit": "ns",
            "value": 30.407663743199418
          },
          {
            "name": "astraweave-quests::batch_operations/validate_batch/500",
            "unit": "ns",
            "value": 305.3852942385765
          },
          {
            "name": "astraweave-quests::component_operations/add_recent_activity",
            "unit": "ns",
            "value": 13.262763214877118
          },
          {
            "name": "astraweave-quests::component_operations/add_remove_active_quest",
            "unit": "ns",
            "value": 13.181229081723757
          },
          {
            "name": "astraweave-quests::component_operations/can_generate_quest",
            "unit": "ns",
            "value": 0.6871715271550406
          },
          {
            "name": "astraweave-quests::component_operations/create_quest_generator",
            "unit": "ns",
            "value": 69.78786529899475
          },
          {
            "name": "astraweave-quests::component_operations/update_context",
            "unit": "ns",
            "value": 36.08539324694209
          },
          {
            "name": "astraweave-quests::quest_completion/check_complete_all_done",
            "unit": "ns",
            "value": 7.171780705695548
          },
          {
            "name": "astraweave-quests::quest_completion/check_complete_none",
            "unit": "ns",
            "value": 0.7633343323757974
          },
          {
            "name": "astraweave-quests::quest_completion/check_complete_partial",
            "unit": "ns",
            "value": 3.4109720585599415
          },
          {
            "name": "astraweave-quests::quest_completion/is_complete_steps/1",
            "unit": "ns",
            "value": 0.9019535213838242
          },
          {
            "name": "astraweave-quests::quest_completion/is_complete_steps/10",
            "unit": "ns",
            "value": 5.195234592456007
          },
          {
            "name": "astraweave-quests::quest_completion/is_complete_steps/100",
            "unit": "ns",
            "value": 32.696597770755844
          },
          {
            "name": "astraweave-quests::quest_completion/is_complete_steps/50",
            "unit": "ns",
            "value": 16.883952782665155
          },
          {
            "name": "astraweave-quests::quest_serialization/deserialize_json/1",
            "unit": "ns",
            "value": 135.8723415468033
          },
          {
            "name": "astraweave-quests::quest_serialization/deserialize_json/10",
            "unit": "ns",
            "value": 737.0204904080985
          },
          {
            "name": "astraweave-quests::quest_serialization/deserialize_json/20",
            "unit": "ns",
            "value": 1578.9053077524825
          },
          {
            "name": "astraweave-quests::quest_serialization/deserialize_json/5",
            "unit": "ns",
            "value": 383.8145172255735
          },
          {
            "name": "astraweave-quests::quest_serialization/deserialize_json/50",
            "unit": "ns",
            "value": 4182.254051063946
          },
          {
            "name": "astraweave-quests::quest_serialization/roundtrip_json/1",
            "unit": "ns",
            "value": 210.7898603498443
          },
          {
            "name": "astraweave-quests::quest_serialization/roundtrip_json/10",
            "unit": "ns",
            "value": 1548.735957093148
          },
          {
            "name": "astraweave-quests::quest_serialization/roundtrip_json/20",
            "unit": "ns",
            "value": 2950.90039928794
          },
          {
            "name": "astraweave-quests::quest_serialization/roundtrip_json/5",
            "unit": "ns",
            "value": 738.9606214127508
          },
          {
            "name": "astraweave-quests::quest_serialization/roundtrip_json/50",
            "unit": "ns",
            "value": 6671.460728769573
          },
          {
            "name": "astraweave-quests::quest_serialization/serialize_json/1",
            "unit": "ns",
            "value": 91.05426214244928
          },
          {
            "name": "astraweave-quests::quest_serialization/serialize_json/10",
            "unit": "ns",
            "value": 576.8783942329611
          },
          {
            "name": "astraweave-quests::quest_serialization/serialize_json/20",
            "unit": "ns",
            "value": 1052.4368521028189
          },
          {
            "name": "astraweave-quests::quest_serialization/serialize_json/5",
            "unit": "ns",
            "value": 335.40678549727613
          },
          {
            "name": "astraweave-quests::quest_serialization/serialize_json/50",
            "unit": "ns",
            "value": 2312.9520263469026
          },
          {
            "name": "astraweave-quests::quest_validation/validate_empty_title",
            "unit": "ns",
            "value": 8.227748525728082
          },
          {
            "name": "astraweave-quests::quest_validation/validate_no_steps",
            "unit": "ns",
            "value": 8.022778706258073
          },
          {
            "name": "astraweave-quests::quest_validation/validate_steps/1",
            "unit": "ns",
            "value": 31.82717326820549
          },
          {
            "name": "astraweave-quests::quest_validation/validate_steps/10",
            "unit": "ns",
            "value": 31.48783891116391
          },
          {
            "name": "astraweave-quests::quest_validation/validate_steps/20",
            "unit": "ns",
            "value": 31.562591389036044
          },
          {
            "name": "astraweave-quests::quest_validation/validate_steps/5",
            "unit": "ns",
            "value": 31.789448303608033
          },
          {
            "name": "astraweave-quests::quest_validation/validate_steps/50",
            "unit": "ns",
            "value": 31.306634095050246
          },
          {
            "name": "astraweave-quests::quest_validation/validate_valid_quest",
            "unit": "ns",
            "value": 1.1144362270931591
          },
          {
            "name": "astraweave-quests::terrain_quests/archetypes_for/Cave",
            "unit": "ns",
            "value": 16.7751254201392
          },
          {
            "name": "astraweave-quests::terrain_quests/archetypes_for/Desert",
            "unit": "ns",
            "value": 16.89798346362843
          },
          {
            "name": "astraweave-quests::terrain_quests/archetypes_for/Forest",
            "unit": "ns",
            "value": 17.427117005001104
          },
          {
            "name": "astraweave-quests::terrain_quests/archetypes_for/Lake",
            "unit": "ns",
            "value": 17.14006663315648
          },
          {
            "name": "astraweave-quests::terrain_quests/archetypes_for/Mountain",
            "unit": "ns",
            "value": 17.828089023621096
          },
          {
            "name": "astraweave-quests::terrain_quests/create_terrain_context",
            "unit": "ns",
            "value": 33.467373679813925
          },
          {
            "name": "astraweave-quests::terrain_quests/get_difficulty_modifier",
            "unit": "ns",
            "value": 8.840654708394574
          },
          {
            "name": "astraweave-quests::terrain_quests/get_quest_archetypes",
            "unit": "ns",
            "value": 17.89989721496773
          },
          {
            "name": "astraweave-quests::terrain_quests/should_trigger_check",
            "unit": "ns",
            "value": 3.2221498569455753
          },
          {
            "name": "astraweave-quests::terrain_quests/should_trigger_no_match",
            "unit": "ns",
            "value": 2.285543531767484
          },
          {
            "name": "astraweave-context::context_window_add_message/10",
            "unit": "ns",
            "value": 7543.076958948816
          },
          {
            "name": "astraweave-context::context_window_add_message/100",
            "unit": "ns",
            "value": 75419.57101885925
          },
          {
            "name": "astraweave-context::context_window_add_message/50",
            "unit": "ns",
            "value": 40671.84501802355
          },
          {
            "name": "astraweave-context::context_window_creation",
            "unit": "ns",
            "value": 224.12487603658127
          },
          {
            "name": "astraweave-context::context_window_with_stats",
            "unit": "ns",
            "value": 31.338316374125963
          },
          {
            "name": "astraweave-context::conversation_history_creation",
            "unit": "ns",
            "value": 218.49182998447898
          },
          {
            "name": "astraweave-context::get_recent_messages/100",
            "unit": "ns",
            "value": 114.5199480016461
          },
          {
            "name": "astraweave-context::get_recent_messages/200",
            "unit": "ns",
            "value": 114.56776168673333
          },
          {
            "name": "astraweave-context::get_recent_messages/50",
            "unit": "ns",
            "value": 114.39056821129041
          },
          {
            "name": "astraweave-context::message_batch_creation/10",
            "unit": "ns",
            "value": 4563.102484917383
          },
          {
            "name": "astraweave-context::message_batch_creation/100",
            "unit": "ns",
            "value": 47850.71332343062
          },
          {
            "name": "astraweave-context::message_batch_creation/50",
            "unit": "ns",
            "value": 23866.18879036034
          },
          {
            "name": "astraweave-context::message_batch_creation/500",
            "unit": "ns",
            "value": 235951.60119756553
          },
          {
            "name": "astraweave-context::message_batch_formatting/10",
            "unit": "ns",
            "value": 738.919980349051
          },
          {
            "name": "astraweave-context::message_batch_formatting/100",
            "unit": "ns",
            "value": 7374.955620412412
          },
          {
            "name": "astraweave-context::message_batch_formatting/50",
            "unit": "ns",
            "value": 3682.1705948204476
          },
          {
            "name": "astraweave-context::message_creation",
            "unit": "ns",
            "value": 364.45115405760436
          },
          {
            "name": "astraweave-context::message_format_for_prompt",
            "unit": "ns",
            "value": 32.75203838880439
          },
          {
            "name": "astraweave-context::window_types/add_50_messages/Fixed",
            "unit": "ns",
            "value": 39229.38900655647
          },
          {
            "name": "astraweave-context::window_types/add_50_messages/Sliding",
            "unit": "ns",
            "value": 40151.25319101016
          },
          {
            "name": "astraweave-terrain::chunk_climate_sampling",
            "unit": "ns",
            "value": 1746072.2696332864
          },
          {
            "name": "astraweave-terrain::climate_sampling",
            "unit": "ns",
            "value": 288.7517634004431
          },
          {
            "name": "astraweave-terrain::heightmap_generation_128x128",
            "unit": "ns",
            "value": 6826495.3875
          },
          {
            "name": "astraweave-terrain::heightmap_generation_128x128_simd",
            "unit": "ns",
            "value": 6861971.275
          },
          {
            "name": "astraweave-terrain::heightmap_generation_64x64",
            "unit": "ns",
            "value": 2024654.4865384612
          },
          {
            "name": "astraweave-terrain::heightmap_generation_64x64_simd",
            "unit": "ns",
            "value": 2035946.5195999998
          },
          {
            "name": "astraweave-terrain::world_chunk_generation",
            "unit": "ns",
            "value": 9343793.621666666
          },
          {
            "name": "astraweave-terrain::world_chunk_generation_with_erosion",
            "unit": "ns",
            "value": 9703279.073333334
          },
          {
            "name": "astraweave-sdk::batch_operations/batched_create/10",
            "unit": "ns",
            "value": 695.2599016667732
          },
          {
            "name": "astraweave-sdk::batch_operations/batched_create/100",
            "unit": "ns",
            "value": 5497.708538029105
          },
          {
            "name": "astraweave-sdk::batch_operations/batched_create/50",
            "unit": "ns",
            "value": 2805.680907130516
          },
          {
            "name": "astraweave-sdk::batch_operations/batched_create/500",
            "unit": "ns",
            "value": 32695.745621947168
          },
          {
            "name": "astraweave-sdk::batch_operations/mixed_batch/10",
            "unit": "ns",
            "value": 451.9613509924052
          },
          {
            "name": "astraweave-sdk::batch_operations/mixed_batch/100",
            "unit": "ns",
            "value": 4540.653364179422
          },
          {
            "name": "astraweave-sdk::batch_operations/mixed_batch/50",
            "unit": "ns",
            "value": 2336.1294431216943
          },
          {
            "name": "astraweave-sdk::batch_operations/mixed_batch/500",
            "unit": "ns",
            "value": 22903.506005354928
          },
          {
            "name": "astraweave-sdk::callback_invocation/heavy_callback_100",
            "unit": "ns",
            "value": 36871.83422079243
          },
          {
            "name": "astraweave-sdk::callback_invocation/invoke_update_callbacks/10",
            "unit": "ns",
            "value": 31.811056035996373
          },
          {
            "name": "astraweave-sdk::callback_invocation/invoke_update_callbacks/100",
            "unit": "ns",
            "value": 230.77303380716702
          },
          {
            "name": "astraweave-sdk::callback_invocation/invoke_update_callbacks/1000",
            "unit": "ns",
            "value": 2229.4468962907818
          },
          {
            "name": "astraweave-sdk::data_marshalling/transform_round_trip_1000",
            "unit": "ns",
            "value": 2562.2238283165334
          },
          {
            "name": "astraweave-sdk::data_marshalling/vec3_round_trip/100",
            "unit": "ns",
            "value": 735.351215262482
          },
          {
            "name": "astraweave-sdk::data_marshalling/vec3_round_trip/1000",
            "unit": "ns",
            "value": 7988.372898930146
          },
          {
            "name": "astraweave-sdk::data_marshalling/vec3_round_trip/10000",
            "unit": "ns",
            "value": 81654.63829175872
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/create_destroy_cycle/100",
            "unit": "ns",
            "value": 7836.022836271532
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/create_destroy_cycle/1000",
            "unit": "ns",
            "value": 83115.71658583463
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/create_destroy_cycle/500",
            "unit": "ns",
            "value": 40924.50867764168
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/transform_update_cycle/100",
            "unit": "ns",
            "value": 3494.0912114367065
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/transform_update_cycle/1000",
            "unit": "ns",
            "value": 35618.61055723886
          },
          {
            "name": "astraweave-sdk::entity_lifecycle/transform_update_cycle/500",
            "unit": "ns",
            "value": 17592.444231876183
          },
          {
            "name": "astraweave-sdk::error_handling/error_code_to_string_1000",
            "unit": "ns",
            "value": 1291.4767798740677
          },
          {
            "name": "astraweave-sdk::error_handling/error_propagation_chain",
            "unit": "ns",
            "value": 192.77815450526458
          },
          {
            "name": "astraweave-sdk::error_handling/error_vs_success_1000",
            "unit": "ns",
            "value": 17878.039811746672
          },
          {
            "name": "astraweave-sdk::ffi_overhead/ffi_with_marshalling",
            "unit": "ns",
            "value": 3.2772481079430746
          },
          {
            "name": "astraweave-sdk::ffi_overhead/ffi_with_ptr_arg",
            "unit": "ns",
            "value": 0.6221620957982983
          },
          {
            "name": "astraweave-sdk::ffi_overhead/minimal_ffi_call",
            "unit": "ns",
            "value": 24.950851865244793
          },
          {
            "name": "astraweave-sdk::handle_operations/handle_creation_10000",
            "unit": "ns",
            "value": 1590.8957586319773
          },
          {
            "name": "astraweave-sdk::handle_operations/handle_lookup_10000",
            "unit": "ns",
            "value": 171707.13661668383
          },
          {
            "name": "astraweave-sdk::handle_operations/handle_validation_10000",
            "unit": "ns",
            "value": 4005.1436810967657
          },
          {
            "name": "astraweave-sdk::json_serialization/snapshot_after_tick",
            "unit": "ns",
            "value": 836.4804600469643
          },
          {
            "name": "astraweave-sdk::json_serialization/snapshot_json_copy",
            "unit": "ns",
            "value": 785.4985752438387
          },
          {
            "name": "astraweave-sdk::json_serialization/snapshot_size_query",
            "unit": "ns",
            "value": 837.2136124936173
          },
          {
            "name": "astraweave-sdk::string_marshalling/cached_string_lookup_1000",
            "unit": "ns",
            "value": 26493.194567148577
          },
          {
            "name": "astraweave-sdk::string_marshalling/cstring_creation",
            "unit": "ns",
            "value": 13.123286494132117
          },
          {
            "name": "astraweave-sdk::string_marshalling/cstring_to_string_1000",
            "unit": "ns",
            "value": 43014.138977518065
          },
          {
            "name": "astraweave-sdk::string_marshalling/cstring_with_format",
            "unit": "ns",
            "value": 28.573722726834717
          },
          {
            "name": "astraweave-sdk::string_marshalling/long_string_marshal_100",
            "unit": "ns",
            "value": 13832.594442960119
          },
          {
            "name": "astraweave-sdk::string_marshalling/string_from_c_buffer",
            "unit": "ns",
            "value": 14.979601219057257
          },
          {
            "name": "astraweave-sdk::string_marshalling/string_to_cstring_1000",
            "unit": "ns",
            "value": 44976.879290786535
          },
          {
            "name": "astraweave-sdk::vector_operations/vec3_length_10000",
            "unit": "ns",
            "value": 6862.091679070132
          },
          {
            "name": "astraweave-sdk::vector_operations/vec3_normalize_10000",
            "unit": "ns",
            "value": 18301.100508133863
          },
          {
            "name": "astraweave-sdk::vector_operations/vec3_operations_combined_10000",
            "unit": "ns",
            "value": 19258.89878293267
          },
          {
            "name": "astraweave-sdk::version_operations/aw_version",
            "unit": "ns",
            "value": 24.71890826181126
          },
          {
            "name": "astraweave-sdk::version_operations/aw_version_string_copy",
            "unit": "ns",
            "value": 3.4261744045329774
          },
          {
            "name": "astraweave-sdk::version_operations/aw_version_string_size",
            "unit": "ns",
            "value": 0.622163605378006
          },
          {
            "name": "astraweave-sdk::world_lifecycle/world_create_destroy",
            "unit": "ns",
            "value": 597.0997828234283
          },
          {
            "name": "astraweave-sdk::world_lifecycle/world_create_only",
            "unit": "ns",
            "value": 1778.2447408911298
          },
          {
            "name": "astraweave-sdk::world_lifecycle/world_destroy",
            "unit": "ns",
            "value": 205.6584815343481
          },
          {
            "name": "astraweave-sdk::world_tick/tick_10_frames",
            "unit": "ns",
            "value": 82.05482841215897
          },
          {
            "name": "astraweave-sdk::world_tick/tick_world",
            "unit": "ns",
            "value": 8.435485731909761
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_communication/gossip_protocol_100_rounds",
            "unit": "ns",
            "value": 26424.91271373759
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_communication/message_broadcast_1000",
            "unit": "ns",
            "value": 1264.5193469064848
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_communication/message_queue_processing_5000",
            "unit": "ns",
            "value": 7975.751058007025
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_communication/range_communication_500",
            "unit": "ns",
            "value": 867.8789112336652
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_decision_making/coalition_formation_200",
            "unit": "ns",
            "value": 44417.057414004936
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_decision_making/consensus_building_100_agents",
            "unit": "ns",
            "value": 2210.5681277782237
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_decision_making/resource_allocation_50_agents",
            "unit": "ns",
            "value": 599.1080553047661
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_decision_making/utility_action_selection_500",
            "unit": "ns",
            "value": 16601.555666656568
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_narrative_coherence/coherence_calculation_50",
            "unit": "ns",
            "value": 25293.22538889489
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_narrative_coherence/conflict_detection_100",
            "unit": "ns",
            "value": 6215.233152444791
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_narrative_coherence/thread_creation_100",
            "unit": "ns",
            "value": 11768.228554643001
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_narrative_coherence/thread_merging_20",
            "unit": "ns",
            "value": 5203.4500693154005
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/faction_clustering_1000",
            "unit": "ns",
            "value": 14510.353635045998
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/graph_construction/100",
            "unit": "ns",
            "value": 22786.962858941006
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/graph_construction/1000",
            "unit": "ns",
            "value": 256016.27118764978
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/graph_construction/500",
            "unit": "ns",
            "value": 125979.69307961149
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/relationship_decay_10000",
            "unit": "ns",
            "value": 7493.774969227675
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/relationship_lookup_10000",
            "unit": "ns",
            "value": 177542.75558490236
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_social_graph/social_path_finding_100",
            "unit": "ns",
            "value": 350208.8374502739
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/communication_propagation",
            "unit": "ns",
            "value": 2524.179861987163
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/formation_positions_20",
            "unit": "ns",
            "value": 83.64903606401771
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/objective_assignment_50_squads",
            "unit": "ns",
            "value": 0.6223073996773169
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/squad_formation/10",
            "unit": "ns",
            "value": 0.6223868299904732
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/squad_formation/20",
            "unit": "ns",
            "value": 0.6223668711585517
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_squad_coordination/squad_formation/5",
            "unit": "ns",
            "value": 0.6222921908126556
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_world_events/agent_involvement_tracking",
            "unit": "ns",
            "value": 67578.59244511754
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_world_events/event_creation_1000",
            "unit": "ns",
            "value": 36351.49299335267
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_world_events/event_filtering_5000",
            "unit": "ns",
            "value": 3284.975889626172
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_world_events/spatial_event_query_1000",
            "unit": "ns",
            "value": 886.4921372325816
          },
          {
            "name": "astraweave-coordination::coordination_adversarial_world_events/timeline_ordering_2000",
            "unit": "ns",
            "value": 1310.0781641943074
          },
          {
            "name": "astraweave-persona::episode_creation",
            "unit": "ns",
            "value": 122.42075294493894
          },
          {
            "name": "astraweave-persona::fact_creation",
            "unit": "ns",
            "value": 38.64205914683059
          },
          {
            "name": "astraweave-persona::persona_creation",
            "unit": "ns",
            "value": 188.8241710533152
          },
          {
            "name": "astraweave-persona::persona_default",
            "unit": "ns",
            "value": 13.93292371331292
          },
          {
            "name": "astraweave-persona::profile_add_facts/10",
            "unit": "ns",
            "value": 1227.2856156213377
          },
          {
            "name": "astraweave-persona::profile_add_facts/100",
            "unit": "ns",
            "value": 16417.294661700642
          },
          {
            "name": "astraweave-persona::profile_add_facts/50",
            "unit": "ns",
            "value": 7753.398145289069
          },
          {
            "name": "astraweave-persona::profile_add_skills/10",
            "unit": "ns",
            "value": 903.3529320935584
          },
          {
            "name": "astraweave-persona::profile_add_skills/100",
            "unit": "ns",
            "value": 11291.213825349098
          },
          {
            "name": "astraweave-persona::profile_add_skills/50",
            "unit": "ns",
            "value": 4621.028526690475
          },
          {
            "name": "astraweave-persona::profile_clone",
            "unit": "ns",
            "value": 8761.075788998894
          },
          {
            "name": "astraweave-persona::profile_creation_comprehensive/profile/f100_s20_e20",
            "unit": "ns",
            "value": 28281.411635917746
          },
          {
            "name": "astraweave-persona::profile_creation_comprehensive/profile/f10_s5_e5",
            "unit": "ns",
            "value": 4157.064948990638
          },
          {
            "name": "astraweave-persona::profile_creation_comprehensive/profile/f50_s10_e10",
            "unit": "ns",
            "value": 10734.044101961308
          },
          {
            "name": "astraweave-persona::profile_creation_default",
            "unit": "ns",
            "value": 29.34672304180846
          },
          {
            "name": "astraweave-persona::profile_deserialize_json",
            "unit": "ns",
            "value": 14281.129191397822
          },
          {
            "name": "astraweave-persona::profile_distill/10",
            "unit": "ns",
            "value": 549.146681736611
          },
          {
            "name": "astraweave-persona::profile_distill/100",
            "unit": "ns",
            "value": 4179.940466108412
          },
          {
            "name": "astraweave-persona::profile_distill/50",
            "unit": "ns",
            "value": 2233.1843566716834
          },
          {
            "name": "astraweave-persona::profile_serialize_json",
            "unit": "ns",
            "value": 6333.804107486658
          },
          {
            "name": "astraweave-persona::profile_sign",
            "unit": "ns",
            "value": 32.00191371078836
          },
          {
            "name": "astraweave-persona::profile_verify",
            "unit": "ns",
            "value": 0.622882870270266
          },
          {
            "name": "astraweave-persona::skill_creation",
            "unit": "ns",
            "value": 72.07382971361146
          },
          {
            "name": "astraweave-behavior::behavior_tree_10_nodes",
            "unit": "ns",
            "value": 108.05079144290758
          },
          {
            "name": "astraweave-behavior::behavior_tree_20_nodes",
            "unit": "ns",
            "value": 368.7081093893653
          },
          {
            "name": "astraweave-behavior::behavior_tree_condition_evaluation",
            "unit": "ns",
            "value": 50.561847593587885
          },
          {
            "name": "astraweave-behavior::behavior_tree_decorator",
            "unit": "ns",
            "value": 32.9147491021957
          },
          {
            "name": "astraweave-behavior::behavior_tree_sequence_evaluation",
            "unit": "ns",
            "value": 96.00126332433001
          },
          {
            "name": "astraweave-behavior::behavior_tree_simple_3_nodes",
            "unit": "ns",
            "value": 55.55895132376703
          },
          {
            "name": "astraweave-behavior::goap_action_preconditions",
            "unit": "ns",
            "value": 11.8411963466658
          },
          {
            "name": "astraweave-behavior::goap_cache_comparison/cache_hit",
            "unit": "ns",
            "value": 406.858382092433
          },
          {
            "name": "astraweave-behavior::goap_cache_comparison/cache_miss",
            "unit": "ns",
            "value": 8441.028301116281
          },
          {
            "name": "astraweave-behavior::goap_caching_cold_cache",
            "unit": "ns",
            "value": 77503.96949373739
          },
          {
            "name": "astraweave-behavior::goap_caching_warm_cache_90pct",
            "unit": "ns",
            "value": 377.3458126905633
          },
          {
            "name": "astraweave-behavior::goap_goal_evaluation",
            "unit": "ns",
            "value": 8.62264654782246
          },
          {
            "name": "astraweave-behavior::goap_planning_10_actions",
            "unit": "ns",
            "value": 609567.5020125116
          },
          {
            "name": "astraweave-behavior::goap_planning_20_actions",
            "unit": "ns",
            "value": 4434433.397500001
          },
          {
            "name": "astraweave-behavior::goap_planning_simple",
            "unit": "ns",
            "value": 1115.3479589288872
          },
          {
            "name": "astraweave-ui::audio_settings/settings_creation",
            "unit": "ns",
            "value": 0.46521521360843165
          },
          {
            "name": "astraweave-ui::audio_settings/volume_adjustment",
            "unit": "ns",
            "value": 4.945410508277124
          },
          {
            "name": "astraweave-ui::controls_settings/key_binding_update",
            "unit": "ns",
            "value": 5.232814625587611
          },
          {
            "name": "astraweave-ui::controls_settings/mouse_sensitivity_adjustment",
            "unit": "ns",
            "value": 1.3524517172048636
          },
          {
            "name": "astraweave-ui::controls_settings/settings_creation",
            "unit": "ns",
            "value": 145.76525536509476
          },
          {
            "name": "astraweave-ui::dialogue_system/dialogue_node_creation",
            "unit": "ns",
            "value": 35.5289751609013
          },
          {
            "name": "astraweave-ui::dialogue_system/dialogue_tree_traversal",
            "unit": "ns",
            "value": 2.0423277794594377
          },
          {
            "name": "astraweave-ui::graphics_settings/quality_preset_change",
            "unit": "ns",
            "value": 1.5960845534914667
          },
          {
            "name": "astraweave-ui::graphics_settings/resolution_update",
            "unit": "ns",
            "value": 3.5894084117014073
          },
          {
            "name": "astraweave-ui::graphics_settings/settings_creation",
            "unit": "ns",
            "value": 3.56042292341543
          },
          {
            "name": "astraweave-ui::hud_manager/damage_number_spawn",
            "unit": "ns",
            "value": 99.86963582670056
          },
          {
            "name": "astraweave-ui::hud_manager/enemy_data_update/1",
            "unit": "ns",
            "value": 21.68822352595591
          },
          {
            "name": "astraweave-ui::hud_manager/enemy_data_update/10",
            "unit": "ns",
            "value": 104.98867099924246
          },
          {
            "name": "astraweave-ui::hud_manager/enemy_data_update/20",
            "unit": "ns",
            "value": 167.30537743804018
          },
          {
            "name": "astraweave-ui::hud_manager/enemy_data_update/5",
            "unit": "ns",
            "value": 61.48111037005434
          },
          {
            "name": "astraweave-ui::hud_manager/enemy_data_update/50",
            "unit": "ns",
            "value": 308.6122948896714
          },
          {
            "name": "astraweave-ui::hud_manager/hud_creation",
            "unit": "ns",
            "value": 14.68015182674888
          },
          {
            "name": "astraweave-ui::hud_manager/player_stats_update",
            "unit": "ns",
            "value": 4.487633306358854
          },
          {
            "name": "astraweave-ui::menu_manager/menu_creation",
            "unit": "ns",
            "value": 2613.354716889229
          },
          {
            "name": "astraweave-ui::menu_manager/settings_navigation",
            "unit": "ns",
            "value": 0.22726720612468515
          },
          {
            "name": "astraweave-ui::menu_manager/state_transitions",
            "unit": "ns",
            "value": 56372.04106714716
          },
          {
            "name": "astraweave-ui::poi_markers/poi_creation/1",
            "unit": "ns",
            "value": 35.32987262800316
          },
          {
            "name": "astraweave-ui::poi_markers/poi_creation/10",
            "unit": "ns",
            "value": 333.39913965707376
          },
          {
            "name": "astraweave-ui::poi_markers/poi_creation/20",
            "unit": "ns",
            "value": 611.6043604960286
          },
          {
            "name": "astraweave-ui::poi_markers/poi_creation/5",
            "unit": "ns",
            "value": 132.04176993778538
          },
          {
            "name": "astraweave-ui::poi_markers/poi_creation/50",
            "unit": "ns",
            "value": 1722.1151253102764
          },
          {
            "name": "astraweave-ui::quest_system/quest_creation",
            "unit": "ns",
            "value": 24.127294039323065
          },
          {
            "name": "astraweave-ui::quest_system/quest_progress_update",
            "unit": "ns",
            "value": 2.672181416126277
          },
          {
            "name": "astraweave-ui::settings_persistence/settings_load",
            "unit": "ns",
            "value": 12636.760993081603
          },
          {
            "name": "astraweave-ui::settings_persistence/settings_save",
            "unit": "ns",
            "value": 279388.11870723736
          },
          {
            "name": "astraweave-ui::tooltip_operations/tooltip_creation",
            "unit": "ns",
            "value": 43.96378356025077
          },
          {
            "name": "astraweave-ui::tooltip_operations/tooltip_with_stats/1",
            "unit": "ns",
            "value": 59.45142645998628
          },
          {
            "name": "astraweave-ui::tooltip_operations/tooltip_with_stats/10",
            "unit": "ns",
            "value": 759.3728946280139
          },
          {
            "name": "astraweave-ui::tooltip_operations/tooltip_with_stats/20",
            "unit": "ns",
            "value": 1613.1781760275483
          },
          {
            "name": "astraweave-ui::tooltip_operations/tooltip_with_stats/5",
            "unit": "ns",
            "value": 378.03993486811896
          },
          {
            "name": "astraweave-ui::ui_adversarial_damage_numbers/arc_motion_100",
            "unit": "ns",
            "value": 0.2359245813745213
          },
          {
            "name": "astraweave-ui::ui_adversarial_damage_numbers/lifetime_filter_200",
            "unit": "ns",
            "value": 315.1297137397628
          },
          {
            "name": "astraweave-ui::ui_adversarial_damage_numbers/shake_calculation_100",
            "unit": "ns",
            "value": 0.23506260031064172
          },
          {
            "name": "astraweave-ui::ui_adversarial_damage_numbers/spawn_burst_50",
            "unit": "ns",
            "value": 114.69039172593227
          },
          {
            "name": "astraweave-ui::ui_adversarial_damage_numbers/spawn_single",
            "unit": "ns",
            "value": 0.4491322587878225
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/easing_calculations_1000",
            "unit": "ns",
            "value": 1600.8111615467144
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/flash_alpha_100",
            "unit": "ns",
            "value": 64.55323955983924
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/multiple_animations/10",
            "unit": "ns",
            "value": 4.968189394071564
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/multiple_animations/100",
            "unit": "ns",
            "value": 60.380362392592225
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/multiple_animations/50",
            "unit": "ns",
            "value": 23.039679852597164
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/rapid_target_changes_100",
            "unit": "ns",
            "value": 321.46373876646135
          },
          {
            "name": "astraweave-ui::ui_adversarial_health_animation/single_update",
            "unit": "ns",
            "value": 0.5996299294817313
          },
          {
            "name": "astraweave-ui::ui_adversarial_layout_calculations/health_bar_width_100",
            "unit": "ns",
            "value": 43.458292437968296
          },
          {
            "name": "astraweave-ui::ui_adversarial_layout_calculations/minimap_transform_100",
            "unit": "ns",
            "value": 44.177454461498975
          },
          {
            "name": "astraweave-ui::ui_adversarial_layout_calculations/screen_space_position_100",
            "unit": "ns",
            "value": 51.44569597685086
          },
          {
            "name": "astraweave-ui::ui_adversarial_layout_calculations/text_size_estimation_50",
            "unit": "ns",
            "value": 26.829591647498226
          },
          {
            "name": "astraweave-ui::ui_adversarial_layout_calculations/visibility_culling_200",
            "unit": "ns",
            "value": 443.4421832151329
          },
          {
            "name": "astraweave-ui::ui_adversarial_menu_state_transitions/equality_checks_1000",
            "unit": "ns",
            "value": 0.23488237090097497
          },
          {
            "name": "astraweave-ui::ui_adversarial_menu_state_transitions/rapid_state_changes_100",
            "unit": "ns",
            "value": 31.145519896079403
          },
          {
            "name": "astraweave-ui::ui_adversarial_menu_state_transitions/state_history_50",
            "unit": "ns",
            "value": 23.264267217288083
          },
          {
            "name": "astraweave-ui::ui_adversarial_menu_state_transitions/state_validation_checks",
            "unit": "ns",
            "value": 0.2240561373245726
          },
          {
            "name": "astraweave-ui::ui_adversarial_quest_tracker/active_quest_filter_50",
            "unit": "ns",
            "value": 146.5384363306708
          },
          {
            "name": "astraweave-ui::ui_adversarial_quest_tracker/completion_calculation_10",
            "unit": "ns",
            "value": 23.91843018847393
          },
          {
            "name": "astraweave-ui::ui_adversarial_quest_tracker/objective_progress_update_100",
            "unit": "ns",
            "value": 50.10602932916604
          },
          {
            "name": "astraweave-ui::ui_adversarial_quest_tracker/quest_lookup_100",
            "unit": "ns",
            "value": 30.808368719568822
          },
          {
            "name": "astraweave-ui::ui_adversarial_settings_validation/audio_validation",
            "unit": "ns",
            "value": 1.0553967761039145
          },
          {
            "name": "astraweave-ui::ui_adversarial_settings_validation/controls_validation",
            "unit": "ns",
            "value": 11.081720019680885
          },
          {
            "name": "astraweave-ui::ui_adversarial_settings_validation/duplicate_key_detection",
            "unit": "ns",
            "value": 267.73713424098355
          },
          {
            "name": "astraweave-ui::ui_adversarial_settings_validation/graphics_validation",
            "unit": "ns",
            "value": 0.5022682202713279
          },
          {
            "name": "astraweave-ui::ui_adversarial_settings_validation/settings_serialization_sim",
            "unit": "ns",
            "value": 214.6527108814011
          },
          {
            "name": "astraweave-pcg::encounter_generation/generate_100_encounters",
            "unit": "ns",
            "value": 11689.824310525819
          },
          {
            "name": "astraweave-pcg::encounter_generation/generate_10_encounters",
            "unit": "ns",
            "value": 690.0776076877011
          },
          {
            "name": "astraweave-pcg::encounter_generation/generate_200_encounters",
            "unit": "ns",
            "value": 33023.9822573622
          },
          {
            "name": "astraweave-pcg::encounter_generation/generate_50_encounters",
            "unit": "ns",
            "value": 4604.856108575514
          },
          {
            "name": "astraweave-pcg::encounter_generation/spacing_check_100",
            "unit": "ns",
            "value": 27.981114194726594
          },
          {
            "name": "astraweave-pcg::full_dungeon_pipeline/huge_dungeon_100r_300e",
            "unit": "ns",
            "value": 77596.48402484096
          },
          {
            "name": "astraweave-pcg::full_dungeon_pipeline/large_dungeon_50r_150e",
            "unit": "ns",
            "value": 25507.397149825334
          },
          {
            "name": "astraweave-pcg::full_dungeon_pipeline/medium_dungeon_20r_50e",
            "unit": "ns",
            "value": 6708.449944634162
          },
          {
            "name": "astraweave-pcg::full_dungeon_pipeline/small_dungeon_5r_10e",
            "unit": "ns",
            "value": 1177.0996698717586
          },
          {
            "name": "astraweave-pcg::rng_operations/choose_from_10",
            "unit": "ns",
            "value": 3.650449116893794
          },
          {
            "name": "astraweave-pcg::rng_operations/create_rng",
            "unit": "ns",
            "value": 35.403171172060794
          },
          {
            "name": "astraweave-pcg::rng_operations/fork_rng",
            "unit": "ns",
            "value": 65.88986701197007
          },
          {
            "name": "astraweave-pcg::rng_operations/gen_bool",
            "unit": "ns",
            "value": 1.760371886340819
          },
          {
            "name": "astraweave-pcg::rng_operations/gen_range_f32",
            "unit": "ns",
            "value": 3.4050711090255077
          },
          {
            "name": "astraweave-pcg::rng_operations/gen_range_i32",
            "unit": "ns",
            "value": 1.850847680054393
          },
          {
            "name": "astraweave-pcg::rng_operations/shuffle_100",
            "unit": "ns",
            "value": 237.01634161120663
          },
          {
            "name": "astraweave-pcg::room_generation/generate_100_rooms",
            "unit": "ns",
            "value": 7170.550467940423
          },
          {
            "name": "astraweave-pcg::room_generation/generate_10_rooms",
            "unit": "ns",
            "value": 608.7958991976843
          },
          {
            "name": "astraweave-pcg::room_generation/generate_20_rooms",
            "unit": "ns",
            "value": 912.1533598042585
          },
          {
            "name": "astraweave-pcg::room_generation/generate_50_rooms",
            "unit": "ns",
            "value": 2512.9595067669347
          },
          {
            "name": "astraweave-pcg::room_generation/generate_5_rooms",
            "unit": "ns",
            "value": 240.86021135718067
          },
          {
            "name": "astraweave-pcg::room_generation/room_center",
            "unit": "ns",
            "value": 0.22290105319538575
          },
          {
            "name": "astraweave-pcg::room_generation/room_overlap_check",
            "unit": "ns",
            "value": 0.23505372841413766
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/100_encounters",
            "unit": "ns",
            "value": 11515.739173428963
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/10_encounters",
            "unit": "ns",
            "value": 784.1837457299979
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/200_encounters",
            "unit": "ns",
            "value": 34309.892900570536
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/25_encounters",
            "unit": "ns",
            "value": 1860.0589024093133
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/500_encounters",
            "unit": "ns",
            "value": 166488.47403054225
          },
          {
            "name": "astraweave-pcg::scaling/encounter_generation/50_encounters",
            "unit": "ns",
            "value": 4453.548012524706
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/100_rooms",
            "unit": "ns",
            "value": 6517.328728170558
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/10_rooms",
            "unit": "ns",
            "value": 440.9828567780462
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/1_rooms",
            "unit": "ns",
            "value": 131.63114275138506
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/20_rooms",
            "unit": "ns",
            "value": 960.9998080174097
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/50_rooms",
            "unit": "ns",
            "value": 2732.1121956321313
          },
          {
            "name": "astraweave-pcg::scaling/room_generation/5_rooms",
            "unit": "ns",
            "value": 250.98226060000547
          },
          {
            "name": "astraweave-security::security_adversarial_access_control/permission_caching_20000",
            "unit": "ns",
            "value": 881119.1574030155
          },
          {
            "name": "astraweave-security::security_adversarial_access_control/rbac_check_50000",
            "unit": "ns",
            "value": 797219.2537558489
          },
          {
            "name": "astraweave-security::security_adversarial_anti_cheat/anomaly_detection_5000",
            "unit": "ns",
            "value": 381048.02815524535
          },
          {
            "name": "astraweave-security::security_adversarial_anti_cheat/cross_reference_check_10000",
            "unit": "ns",
            "value": 1070065.3040997963
          },
          {
            "name": "astraweave-security::security_adversarial_anti_cheat/event_logging_50000",
            "unit": "ns",
            "value": 10807651.176000005
          },
          {
            "name": "astraweave-security::security_adversarial_anti_cheat/metrics_analysis_10000",
            "unit": "ns",
            "value": 14983.624778624979
          },
          {
            "name": "astraweave-security::security_adversarial_anti_cheat/movement_validation_20000",
            "unit": "ns",
            "value": 435368.10357717134
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/content_sanitization_5000",
            "unit": "ns",
            "value": 1541301.5794014889
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/multi_category_filter_10000",
            "unit": "ns",
            "value": 774644.5079306442
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/pattern_matching/10",
            "unit": "ns",
            "value": 32605.42579331726
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/pattern_matching/100",
            "unit": "ns",
            "value": 311821.22155968414
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/pattern_matching/50",
            "unit": "ns",
            "value": 159568.75438562574
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/pattern_matching/500",
            "unit": "ns",
            "value": 1572302.76716257
          },
          {
            "name": "astraweave-security::security_adversarial_content_filtering/pii_detection_5000",
            "unit": "ns",
            "value": 130905.38953068799
          },
          {
            "name": "astraweave-security::security_adversarial_input_validation/numeric_validation_50000",
            "unit": "ns",
            "value": 34072.614233914006
          },
          {
            "name": "astraweave-security::security_adversarial_input_validation/path_traversal_check_10000",
            "unit": "ns",
            "value": 840886.7065934902
          },
          {
            "name": "astraweave-security::security_adversarial_input_validation/schema_validation_5000",
            "unit": "ns",
            "value": 233758.2934983007
          },
          {
            "name": "astraweave-security::security_adversarial_input_validation/string_sanitization_10000",
            "unit": "ns",
            "value": 2870813.662222221
          },
          {
            "name": "astraweave-security::security_adversarial_llm_validation/injection_detection_10000",
            "unit": "ns",
            "value": 383820.5143899347
          },
          {
            "name": "astraweave-security::security_adversarial_llm_validation/rate_limiting_20000",
            "unit": "ns",
            "value": 476563.8462601001
          },
          {
            "name": "astraweave-security::security_adversarial_llm_validation/risk_scoring_10000",
            "unit": "ns",
            "value": 598399.8283554275
          },
          {
            "name": "astraweave-security::security_adversarial_llm_validation/token_budget_enforcement_10000",
            "unit": "ns",
            "value": 479358.71203253756
          },
          {
            "name": "astraweave-security::security_adversarial_script_sandboxing/call_depth_tracking_50000",
            "unit": "ns",
            "value": 157574.28431039967
          },
          {
            "name": "astraweave-security::security_adversarial_script_sandboxing/memory_tracking_10000",
            "unit": "ns",
            "value": 1441548.1654452658
          },
          {
            "name": "astraweave-security::security_adversarial_script_sandboxing/module_access_check_50000",
            "unit": "ns",
            "value": 432436.9770386324
          },
          {
            "name": "astraweave-security::security_adversarial_script_sandboxing/operation_counting_100000",
            "unit": "ns",
            "value": 3.1231251403150524
          },
          {
            "name": "astraweave-security::security_adversarial_script_sandboxing/sandbox_config_creation_10000",
            "unit": "ns",
            "value": 1811849.658548239
          },
          {
            "name": "astraweave-stress-test::ecs_performance",
            "unit": "ns",
            "value": 275506.72124926443
          },
          {
            "name": "astraweave-stress-test::network_stress",
            "unit": "ns",
            "value": 147584.84696214896
          },
          {
            "name": "astraweave-stress-test::persistence_stress",
            "unit": "ns",
            "value": 568622.774135444
          },
          {
            "name": "astraweave-ai::GOAP Planning/1 enemy (simple)",
            "unit": "ns",
            "value": 59.144298181123425
          },
          {
            "name": "astraweave-ai::GOAP Planning/10 enemies + 5 POIs (complex)",
            "unit": "ns",
            "value": 58.87091566536887
          },
          {
            "name": "astraweave-ai::GOAP Planning/3 enemies + 2 POIs (moderate)",
            "unit": "ns",
            "value": 58.463639722515246
          },
          {
            "name": "astraweave-ai::Multi-Agent Throughput/10",
            "unit": "ns",
            "value": 561.4823609472218
          },
          {
            "name": "astraweave-ai::Multi-Agent Throughput/100",
            "unit": "ns",
            "value": 5550.1995255522015
          },
          {
            "name": "astraweave-ai::Multi-Agent Throughput/50",
            "unit": "ns",
            "value": 2774.242825847516
          },
          {
            "name": "astraweave-ai::Multi-Agent Throughput/500",
            "unit": "ns",
            "value": 27680.919672734937
          },
          {
            "name": "astraweave-ai::Orchestrator Comparison/GOAP",
            "unit": "ns",
            "value": 61.43238949025934
          },
          {
            "name": "astraweave-ai::Orchestrator Comparison/Rule-based",
            "unit": "ns",
            "value": 69.28438904849001
          },
          {
            "name": "astraweave-ai::Orchestrator Comparison/Utility",
            "unit": "ns",
            "value": 66.53169605840654
          },
          {
            "name": "astraweave-ai::Planning Conditions/low ammo (3 enemies)",
            "unit": "ns",
            "value": 58.541716502872696
          },
          {
            "name": "astraweave-ai::Planning Conditions/low morale (5 enemies)",
            "unit": "ns",
            "value": 59.28746560477861
          },
          {
            "name": "astraweave-ai::Planning Conditions/no enemies (idle)",
            "unit": "ns",
            "value": 46.83779220951733
          },
          {
            "name": "astraweave-ai::Rule-Based Planning/1 enemy",
            "unit": "ns",
            "value": 74.96241345175484
          },
          {
            "name": "astraweave-ai::Rule-Based Planning/3 enemies",
            "unit": "ns",
            "value": 76.05467881145537
          },
          {
            "name": "astraweave-ai::Tool Validation/validate CoverFire",
            "unit": "ns",
            "value": 98.02111791838394
          },
          {
            "name": "astraweave-ai::Tool Validation/validate MoveTo",
            "unit": "ns",
            "value": 61.673832520137076
          },
          {
            "name": "astraweave-ai::Utility AI Planning/1 enemy",
            "unit": "ns",
            "value": 70.93594691230024
          },
          {
            "name": "astraweave-ai::Utility AI Planning/complex scenario",
            "unit": "ns",
            "value": 66.23309817005644
          },
          {
            "name": "astraweave-ai::WorldSnapshot Operations/clone complex snapshot",
            "unit": "ns",
            "value": 240.56093684131855
          },
          {
            "name": "astraweave-ai::WorldSnapshot Operations/clone large snapshot (100 enemies)",
            "unit": "ns",
            "value": 1997.4802622860993
          },
          {
            "name": "astraweave-ai::WorldSnapshot Operations/clone simple snapshot",
            "unit": "ns",
            "value": 59.52701104837797
          },
          {
            "name": "astraweave-ai::ai_loop_full_end_to_end_complex",
            "unit": "ns",
            "value": 794.9397877951419
          },
          {
            "name": "astraweave-ai::ai_loop_full_end_to_end_moderate",
            "unit": "ns",
            "value": 308.31729620569513
          },
          {
            "name": "astraweave-ai::ai_loop_full_end_to_end_simple",
            "unit": "ns",
            "value": 55.04533600781937
          },
          {
            "name": "astraweave-ai::ai_loop_plan_validation",
            "unit": "ns",
            "value": 82.6445710943719
          },
          {
            "name": "astraweave-ai::ai_loop_rule_planner_complex",
            "unit": "ns",
            "value": 88.92179380331136
          },
          {
            "name": "astraweave-ai::ai_loop_rule_planner_moderate",
            "unit": "ns",
            "value": 82.83649123832835
          },
          {
            "name": "astraweave-ai::ai_loop_rule_planner_simple",
            "unit": "ns",
            "value": 45.09175296722226
          },
          {
            "name": "astraweave-ai::ai_loop_snapshot_creation_complex",
            "unit": "ns",
            "value": 711.1010537961587
          },
          {
            "name": "astraweave-ai::ai_loop_snapshot_creation_moderate",
            "unit": "ns",
            "value": 235.13119729987864
          },
          {
            "name": "astraweave-ai::ai_loop_snapshot_creation_simple",
            "unit": "ns",
            "value": 27.617362685662087
          },
          {
            "name": "astraweave-ai::feedback_phase/100_agents",
            "unit": "ns",
            "value": 68.58161511687457
          },
          {
            "name": "astraweave-ai::full_multi_agent_pipeline/large_100a_5e",
            "unit": "ns",
            "value": 30025.655900842023
          },
          {
            "name": "astraweave-ai::full_multi_agent_pipeline/medium_50a_5e",
            "unit": "ns",
            "value": 15271.753581880126
          },
          {
            "name": "astraweave-ai::full_multi_agent_pipeline/small_10a_3e",
            "unit": "ns",
            "value": 2698.671128612058
          },
          {
            "name": "astraweave-ai::full_multi_agent_pipeline/stress_500a_5e",
            "unit": "ns",
            "value": 153496.06565299816
          },
          {
            "name": "astraweave-ai::goap_feature_disabled",
            "unit": "ns",
            "value": 0.6224888795067254
          },
          {
            "name": "astraweave-ai::goap_next_action_close",
            "unit": "ns",
            "value": 3.7402463361483758
          },
          {
            "name": "astraweave-ai::goap_next_action_far",
            "unit": "ns",
            "value": 5.30121824543334
          },
          {
            "name": "astraweave-ai::goap_next_action_no_enemies",
            "unit": "ns",
            "value": 3.4245840270785535
          },
          {
            "name": "astraweave-ai::goap_propose_plan_close",
            "unit": "ns",
            "value": 54.632458570302326
          },
          {
            "name": "astraweave-ai::goap_propose_plan_far",
            "unit": "ns",
            "value": 56.18568449829207
          },
          {
            "name": "astraweave-ai::htn_expansions_per_tick",
            "unit": "ns",
            "value": 11862.144207603546
          },
          {
            "name": "astraweave-ai::integration_pipeline_rule/rule_full_pipeline/1",
            "unit": "ns",
            "value": 74.26294201965423
          },
          {
            "name": "astraweave-ai::integration_pipeline_rule/rule_full_pipeline/10",
            "unit": "ns",
            "value": 74.24137303095955
          },
          {
            "name": "astraweave-ai::integration_pipeline_rule/rule_full_pipeline/100",
            "unit": "ns",
            "value": 74.1439729215088
          },
          {
            "name": "astraweave-ai::integration_pipeline_rule/rule_full_pipeline/50",
            "unit": "ns",
            "value": 74.11212380439122
          },
          {
            "name": "astraweave-ai::integration_pipeline_rule/rule_full_pipeline/500",
            "unit": "ns",
            "value": 74.57979984682176
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/1",
            "unit": "ns",
            "value": 202.88052580994682
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/10",
            "unit": "ns",
            "value": 2711.1350883683813
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/100",
            "unit": "ns",
            "value": 30981.466004620608
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/1000",
            "unit": "ns",
            "value": 309281.42017393117
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/200",
            "unit": "ns",
            "value": 61866.15222813322
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/50",
            "unit": "ns",
            "value": 15813.620574253513
          },
          {
            "name": "astraweave-ai::multi_agent_scaling/500",
            "unit": "ns",
            "value": 154817.00342134782
          },
          {
            "name": "astraweave-ai::per_agent_latency/1000_agents",
            "unit": "ns",
            "value": 4.073189512122839
          },
          {
            "name": "astraweave-ai::per_agent_latency/100_agents",
            "unit": "ns",
            "value": 0.5024766417738522
          },
          {
            "name": "astraweave-ai::per_agent_latency/10_agents",
            "unit": "ns",
            "value": 0.036947672594759104
          },
          {
            "name": "astraweave-ai::per_agent_pipeline_overhead",
            "unit": "ns",
            "value": 74.70805409226524
          },
          {
            "name": "astraweave-ai::perception_phase/100_agents_5_enemies",
            "unit": "ns",
            "value": 15645.1065979153
          },
          {
            "name": "astraweave-ai::perception_phase/10_agents_3_enemies",
            "unit": "ns",
            "value": 1368.05784283483
          },
          {
            "name": "astraweave-ai::perception_phase/500_agents_5_enemies",
            "unit": "ns",
            "value": 78707.8589611731
          },
          {
            "name": "astraweave-ai::perception_phase/50_agents_5_enemies",
            "unit": "ns",
            "value": 7920.173321913206
          },
          {
            "name": "astraweave-ai::planner_advanced_feature_disabled",
            "unit": "ns",
            "value": 0.6230839588738277
          },
          {
            "name": "astraweave-ai::planning_phase/100_agents",
            "unit": "ns",
            "value": 13115.299162736712
          },
          {
            "name": "astraweave-ai::planning_phase/10_agents",
            "unit": "ns",
            "value": 901.8755811355413
          },
          {
            "name": "astraweave-ai::planning_phase/500_agents",
            "unit": "ns",
            "value": 69868.19247045815
          },
          {
            "name": "astraweave-ai::planning_phase/50_agents",
            "unit": "ns",
            "value": 6589.842245359997
          },
          {
            "name": "astraweave-ai::scalability_analysis/rule_scaling/10",
            "unit": "ns",
            "value": 74.89469729318415
          },
          {
            "name": "astraweave-ai::scalability_analysis/rule_scaling/100",
            "unit": "ns",
            "value": 74.41393221242087
          },
          {
            "name": "astraweave-ai::scalability_analysis/rule_scaling/200",
            "unit": "ns",
            "value": 74.50231772244798
          },
          {
            "name": "astraweave-ai::scalability_analysis/rule_scaling/50",
            "unit": "ns",
            "value": 74.36931613834295
          },
          {
            "name": "astraweave-ai::scalability_analysis/rule_scaling/500",
            "unit": "ns",
            "value": 74.44571773458715
          },
          {
            "name": "astraweave-ai::snapshot_creation_scalable/create_snapshot/10",
            "unit": "ns",
            "value": 418.0054413142509
          },
          {
            "name": "astraweave-ai::snapshot_creation_scalable/create_snapshot/100",
            "unit": "ns",
            "value": 743.7511956528178
          },
          {
            "name": "astraweave-ai::snapshot_creation_scalable/create_snapshot/50",
            "unit": "ns",
            "value": 650.2274093967817
          },
          {
            "name": "astraweave-ai::snapshot_creation_scalable/create_snapshot/500",
            "unit": "ns",
            "value": 1863.1985297208369
          },
          {
            "name": "astraweave-ai::validation_phase/100_plans",
            "unit": "ns",
            "value": 27.813735156483776
          },
          {
            "name": "astraweave-render@a::AABB Construction/aabb_from_transform_identity",
            "unit": "ns",
            "value": 10.324460534933184
          },
          {
            "name": "astraweave-render@a::AABB Construction/aabb_from_transform_rotated",
            "unit": "ns",
            "value": 10.385265327421177
          },
          {
            "name": "astraweave-render@a::AABB Construction/aabb_new",
            "unit": "ns",
            "value": 10.950429971056312
          },
          {
            "name": "astraweave-render@a::AABB Construction/batch/100",
            "unit": "ns",
            "value": 1074.8060128622735
          },
          {
            "name": "astraweave-render@a::AABB Construction/batch/1000",
            "unit": "ns",
            "value": 10554.58009232703
          },
          {
            "name": "astraweave-render@a::AABB Construction/batch/10000",
            "unit": "ns",
            "value": 105883.80413365133
          },
          {
            "name": "astraweave-render@a::AABB-Frustum Test/single_boundary",
            "unit": "ns",
            "value": 17.831777970657743
          },
          {
            "name": "astraweave-render@a::AABB-Frustum Test/single_culled",
            "unit": "ns",
            "value": 13.273818663873168
          },
          {
            "name": "astraweave-render@a::AABB-Frustum Test/single_visible",
            "unit": "ns",
            "value": 17.828141674749126
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/10pct_visible/100",
            "unit": "ns",
            "value": 543.6932276678826
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/10pct_visible/1000",
            "unit": "ns",
            "value": 5352.558029077992
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/10pct_visible/10000",
            "unit": "ns",
            "value": 53882.288539417226
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/10pct_visible/50000",
            "unit": "ns",
            "value": 269098.5668572071
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/50pct_visible/100",
            "unit": "ns",
            "value": 549.7589874834943
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/50pct_visible/1000",
            "unit": "ns",
            "value": 5351.072189938666
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/50pct_visible/10000",
            "unit": "ns",
            "value": 53647.1868780802
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/50pct_visible/50000",
            "unit": "ns",
            "value": 269354.64667627943
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/90pct_visible/100",
            "unit": "ns",
            "value": 1197.8543718290705
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/90pct_visible/1000",
            "unit": "ns",
            "value": 10184.169848881527
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/90pct_visible/10000",
            "unit": "ns",
            "value": 98458.92431613599
          },
          {
            "name": "astraweave-render@a::CPU Frustum Culling/90pct_visible/50000",
            "unit": "ns",
            "value": 488194.8113455724
          },
          {
            "name": "astraweave-render@a::Edge Collapses/build/1000v_1500t",
            "unit": "ns",
            "value": 97058.3845918166
          },
          {
            "name": "astraweave-render@a::Edge Collapses/build/100v_150t",
            "unit": "ns",
            "value": 7959.862085337853
          },
          {
            "name": "astraweave-render@a::Edge Collapses/build/500v_750t",
            "unit": "ns",
            "value": 47040.319014764784
          },
          {
            "name": "astraweave-render@a::Frustum Extraction/from_view_proj",
            "unit": "ns",
            "value": 17.792246130822008
          },
          {
            "name": "astraweave-render@a::Full Culling Pipeline/cull_and_batch/1000",
            "unit": "ns",
            "value": 5108.818952074282
          },
          {
            "name": "astraweave-render@a::Full Culling Pipeline/cull_and_batch/10000",
            "unit": "ns",
            "value": 50758.000246736214
          },
          {
            "name": "astraweave-render@a::Full Culling Pipeline/cull_and_batch/5000",
            "unit": "ns",
            "value": 25406.79790120964
          },
          {
            "name": "astraweave-render@a::Indirect Commands/build_commands/10",
            "unit": "ns",
            "value": 19.301621111630077
          },
          {
            "name": "astraweave-render@a::Indirect Commands/build_commands/100",
            "unit": "ns",
            "value": 123.45994544512128
          },
          {
            "name": "astraweave-render@a::Indirect Commands/build_commands/50",
            "unit": "ns",
            "value": 50.41533209929793
          },
          {
            "name": "astraweave-render@a::Indirect Commands/build_commands/500",
            "unit": "ns",
            "value": 441.9804078394841
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/25pct/1000v",
            "unit": "ns",
            "value": 141505.1750136681
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/25pct/2000v",
            "unit": "ns",
            "value": 297883.1182033365
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/25pct/500v",
            "unit": "ns",
            "value": 66505.37138601766
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/50pct/1000v",
            "unit": "ns",
            "value": 131946.58038885606
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/50pct/2000v",
            "unit": "ns",
            "value": 272968.38541271444
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/50pct/500v",
            "unit": "ns",
            "value": 62360.890119495605
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/75pct/1000v",
            "unit": "ns",
            "value": 120558.5928030748
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/75pct/2000v",
            "unit": "ns",
            "value": 245476.35609081658
          },
          {
            "name": "astraweave-render@a::Mesh Simplification/75pct/500v",
            "unit": "ns",
            "value": 56945.63844991789
          },
          {
            "name": "astraweave-render@a::Quadric Operations/add",
            "unit": "ns",
            "value": 2.235833124833979
          },
          {
            "name": "astraweave-render@a::Quadric Operations/evaluate",
            "unit": "ns",
            "value": 4.017094864782423
          },
          {
            "name": "astraweave-render@a::Quadric Operations/from_plane",
            "unit": "ns",
            "value": 2.8042656466616545
          },
          {
            "name": "astraweave-render@a::Vertex Quadrics/compute/1000v_1500t",
            "unit": "ns",
            "value": 7332.828834545254
          },
          {
            "name": "astraweave-render@a::Vertex Quadrics/compute/100v_150t",
            "unit": "ns",
            "value": 808.6587247684038
          },
          {
            "name": "astraweave-render@a::Vertex Quadrics/compute/5000v_7500t",
            "unit": "ns",
            "value": 36873.28423903481
          },
          {
            "name": "astraweave-render@a::Vertex Quadrics/compute/500v_750t",
            "unit": "ns",
            "value": 3671.8421730878968
          },
          {
            "name": "astraweave-render@a::biome_pipeline/1000_biome_lookups",
            "unit": "ns",
            "value": 126231.91750977976
          },
          {
            "name": "astraweave-render@a::biome_pipeline/biome_change_with_weather",
            "unit": "ns",
            "value": 48.060646222173844
          },
          {
            "name": "astraweave-render@a::biome_pipeline/per_frame_during_transition",
            "unit": "ns",
            "value": 18.107433145971296
          },
          {
            "name": "astraweave-render@a::biome_pipeline/per_frame_steady_state",
            "unit": "ns",
            "value": 4.0481179084586225
          },
          {
            "name": "astraweave-render@a::biome_scaling/parallel_zones/1",
            "unit": "ns",
            "value": 50.465051323205564
          },
          {
            "name": "astraweave-render@a::biome_scaling/parallel_zones/16",
            "unit": "ns",
            "value": 796.8627207211449
          },
          {
            "name": "astraweave-render@a::biome_scaling/parallel_zones/4",
            "unit": "ns",
            "value": 201.4707401552519
          },
          {
            "name": "astraweave-render@a::biome_scaling/parallel_zones/64",
            "unit": "ns",
            "value": 3169.0304041281183
          },
          {
            "name": "astraweave-render@a::biome_visuals/for_biome_all_8",
            "unit": "ns",
            "value": 126.08293121131892
          },
          {
            "name": "astraweave-render@a::biome_visuals/for_biome_single",
            "unit": "ns",
            "value": 15.846813175709169
          },
          {
            "name": "astraweave-render@a::biome_visuals/lerp_full_transition_100_steps",
            "unit": "ns",
            "value": 2979.7738587794297
          },
          {
            "name": "astraweave-render@a::biome_visuals/lerp_single",
            "unit": "ns",
            "value": 30.35114004729352
          },
          {
            "name": "astraweave-render@a::bloom_mip/downsample/mip0",
            "unit": "ns",
            "value": 59009858.84
          },
          {
            "name": "astraweave-render@a::bloom_mip/downsample/mip1",
            "unit": "ns",
            "value": 14758204.5075
          },
          {
            "name": "astraweave-render@a::bloom_mip/downsample/mip2",
            "unit": "ns",
            "value": 3692447.2164285723
          },
          {
            "name": "astraweave-render@a::bloom_mip/downsample/mip3",
            "unit": "ns",
            "value": 923864.814598351
          },
          {
            "name": "astraweave-render@a::bloom_mip/downsample/mip4",
            "unit": "ns",
            "value": 228235.34619957558
          },
          {
            "name": "astraweave-render@a::bloom_mip/upsample/mip0",
            "unit": "ns",
            "value": 102790600.17
          },
          {
            "name": "astraweave-render@a::bloom_mip/upsample/mip1",
            "unit": "ns",
            "value": 25714883.255
          },
          {
            "name": "astraweave-render@a::bloom_mip/upsample/mip2",
            "unit": "ns",
            "value": 6431438.465
          },
          {
            "name": "astraweave-render@a::bloom_mip/upsample/mip3",
            "unit": "ns",
            "value": 1606377.6220200928
          },
          {
            "name": "astraweave-render@a::bloom_mip/upsample/mip4",
            "unit": "ns",
            "value": 397457.2927842728
          },
          {
            "name": "astraweave-render@a::bloom_threshold/extract/1080p",
            "unit": "ns",
            "value": 3699928.014285712
          },
          {
            "name": "astraweave-render@a::bloom_threshold/extract/1440p",
            "unit": "ns",
            "value": 6575866.97
          },
          {
            "name": "astraweave-render@a::bloom_threshold/extract/4K",
            "unit": "ns",
            "value": 14796690.9575
          },
          {
            "name": "astraweave-render@a::bloom_threshold/extract/720p",
            "unit": "ns",
            "value": 1644585.7247505032
          },
          {
            "name": "astraweave-render@a::camera_controller/new",
            "unit": "ns",
            "value": 11.588661165233612
          },
          {
            "name": "astraweave-render@a::camera_controller/process_keyboard",
            "unit": "ns",
            "value": 3.1122363749797755
          },
          {
            "name": "astraweave-render@a::camera_controller/process_mouse_delta",
            "unit": "ns",
            "value": 1.6501804095959032
          },
          {
            "name": "astraweave-render@a::camera_controller/process_scroll_freefly",
            "unit": "ns",
            "value": 1.5566722637998045
          },
          {
            "name": "astraweave-render@a::camera_controller/process_scroll_orbit",
            "unit": "ns",
            "value": 1.8749984871442145
          },
          {
            "name": "astraweave-render@a::camera_controller/toggle_mode",
            "unit": "ns",
            "value": 0.7861340421887851
          },
          {
            "name": "astraweave-render@a::camera_controller/update_freefly",
            "unit": "ns",
            "value": 41.70424166747521
          },
          {
            "name": "astraweave-render@a::camera_controller/update_orbit",
            "unit": "ns",
            "value": 17.423823478428723
          },
          {
            "name": "astraweave-render@a::camera_operations/direction_batch_16",
            "unit": "ns",
            "value": 233.38654309818054
          },
          {
            "name": "astraweave-render@a::camera_operations/direction_calc",
            "unit": "ns",
            "value": 11.637887984797958
          },
          {
            "name": "astraweave-render@a::camera_operations/proj_matrix",
            "unit": "ns",
            "value": 2.500349588580023
          },
          {
            "name": "astraweave-render@a::camera_operations/view_matrix",
            "unit": "ns",
            "value": 2.491840634047426
          },
          {
            "name": "astraweave-render@a::camera_operations/view_projection",
            "unit": "ns",
            "value": 41.54685575653846
          },
          {
            "name": "astraweave-render@a::combined_scenarios/atlas_populate_terrain_set",
            "unit": "ns",
            "value": 255.11956985624437
          },
          {
            "name": "astraweave-render@a::combined_scenarios/cinematic_camera_sequence",
            "unit": "ns",
            "value": 5806.873665441971
          },
          {
            "name": "astraweave-render@a::combined_scenarios/full_pipeline_init",
            "unit": "ns",
            "value": 24426.073155687143
          },
          {
            "name": "astraweave-render@a::combined_scenarios/large_forest_10k_trees",
            "unit": "ns",
            "value": 371582.25943396223
          },
          {
            "name": "astraweave-render@a::combined_scenarios/material_batch_load_24",
            "unit": "ns",
            "value": 6073.130431823805
          },
          {
            "name": "astraweave-render@a::combined_scenarios/mesh_batch_generate_lod_chain",
            "unit": "ns",
            "value": 116738.94922155689
          },
          {
            "name": "astraweave-render@a::combined_scenarios/scene_setup_mixed",
            "unit": "ns",
            "value": 6114.968451230913
          },
          {
            "name": "astraweave-render@a::combined_scenarios/spawn_wave_32x32_grid",
            "unit": "ns",
            "value": 3355.3799370466672
          },
          {
            "name": "astraweave-render@a::combined_scenarios/typical_frame_100_instances",
            "unit": "ns",
            "value": 655.723404981235
          },
          {
            "name": "astraweave-render@a::combined_scenarios/typical_frame_setup",
            "unit": "ns",
            "value": 451.918070975506
          },
          {
            "name": "astraweave-render@a::cpu_cluster_binning_1k_lights",
            "unit": "ns",
            "value": 142889.06514438384
          },
          {
            "name": "astraweave-render@a::csm_cascade/split_calculation/2",
            "unit": "ns",
            "value": 53.17183739157512
          },
          {
            "name": "astraweave-render@a::csm_cascade/split_calculation/4",
            "unit": "ns",
            "value": 62.67098343928697
          },
          {
            "name": "astraweave-render@a::csm_cascade/split_calculation/6",
            "unit": "ns",
            "value": 86.92606331470614
          },
          {
            "name": "astraweave-render@a::csm_cascade/split_calculation/8",
            "unit": "ns",
            "value": 97.7541122874855
          },
          {
            "name": "astraweave-render@a::csm_matrix/ortho_projection/cascade0",
            "unit": "ns",
            "value": 7.885091418751788
          },
          {
            "name": "astraweave-render@a::csm_matrix/ortho_projection/cascade1",
            "unit": "ns",
            "value": 7.960908183847856
          },
          {
            "name": "astraweave-render@a::csm_matrix/ortho_projection/cascade2",
            "unit": "ns",
            "value": 7.952090675256755
          },
          {
            "name": "astraweave-render@a::csm_matrix/ortho_projection/cascade3",
            "unit": "ns",
            "value": 7.8781815838767475
          },
          {
            "name": "astraweave-render@a::culling_performance/with_backface_culling",
            "unit": "ns",
            "value": 0.311501435359254
          },
          {
            "name": "astraweave-render@a::culling_performance/without_backface_culling",
            "unit": "ns",
            "value": 0.3115744380532278
          },
          {
            "name": "astraweave-render@a::instance_batching/add_instances/10",
            "unit": "ns",
            "value": 130.0907149780825
          },
          {
            "name": "astraweave-render@a::instance_batching/add_instances/100",
            "unit": "ns",
            "value": 558.6627324874962
          },
          {
            "name": "astraweave-render@a::instance_batching/add_instances/1000",
            "unit": "ns",
            "value": 3109.981972820848
          },
          {
            "name": "astraweave-render@a::instance_batching/add_instances/5000",
            "unit": "ns",
            "value": 13336.892699263228
          },
          {
            "name": "astraweave-render@a::instance_batching/batch_new",
            "unit": "ns",
            "value": 2.800296733811894
          },
          {
            "name": "astraweave-render@a::instance_batching/to_raw_data/100",
            "unit": "ns",
            "value": 572.6021418292752
          },
          {
            "name": "astraweave-render@a::instance_batching/to_raw_data/1000",
            "unit": "ns",
            "value": 5395.383396780509
          },
          {
            "name": "astraweave-render@a::instance_batching/to_raw_data/5000",
            "unit": "ns",
            "value": 27223.84013605442
          },
          {
            "name": "astraweave-render@a::instance_manager/add/1000meshes_x_1",
            "unit": "ns",
            "value": 103307.69137380192
          },
          {
            "name": "astraweave-render@a::instance_manager/add/100meshes_x_10",
            "unit": "ns",
            "value": 36869.64140919367
          },
          {
            "name": "astraweave-render@a::instance_manager/add/10meshes_x_100",
            "unit": "ns",
            "value": 22146.235420060508
          },
          {
            "name": "astraweave-render@a::instance_manager/add/1meshes_x_1000",
            "unit": "ns",
            "value": 18128.07837584396
          },
          {
            "name": "astraweave-render@a::instance_manager/calculate_savings_1000_instances_10_meshes",
            "unit": "ns",
            "value": 0.6352567091001581
          },
          {
            "name": "astraweave-render@a::instance_manager/clear_1000_instances",
            "unit": "ns",
            "value": 297.1354515886915
          },
          {
            "name": "astraweave-render@a::instance_manager/new",
            "unit": "ns",
            "value": 5.642628037710464
          },
          {
            "name": "astraweave-render@a::instance_patterns/circle/128",
            "unit": "ns",
            "value": 2215.5848725009514
          },
          {
            "name": "astraweave-render@a::instance_patterns/circle/32",
            "unit": "ns",
            "value": 699.3533127258355
          },
          {
            "name": "astraweave-render@a::instance_patterns/circle/512",
            "unit": "ns",
            "value": 8410.462417657627
          },
          {
            "name": "astraweave-render@a::instance_patterns/circle/8",
            "unit": "ns",
            "value": 208.4624845529964
          },
          {
            "name": "astraweave-render@a::instance_patterns/grid/100x100",
            "unit": "ns",
            "value": 37062.69063444108
          },
          {
            "name": "astraweave-render@a::instance_patterns/grid/10x10",
            "unit": "ns",
            "value": 600.6564255176979
          },
          {
            "name": "astraweave-render@a::instance_patterns/grid/32x32",
            "unit": "ns",
            "value": 3248.9147116580475
          },
          {
            "name": "astraweave-render@a::instance_patterns/random_scatter/100",
            "unit": "ns",
            "value": 2393.9620358644447
          },
          {
            "name": "astraweave-render@a::instance_patterns/random_scatter/1000",
            "unit": "ns",
            "value": 21412.746582733813
          },
          {
            "name": "astraweave-render@a::instance_patterns/random_scatter/5000",
            "unit": "ns",
            "value": 111075.21101265823
          },
          {
            "name": "astraweave-render@a::instance_transforms/new_identity",
            "unit": "ns",
            "value": 7.421283560075021
          },
          {
            "name": "astraweave-render@a::instance_transforms/new_positioned",
            "unit": "ns",
            "value": 8.32778114941532
          },
          {
            "name": "astraweave-render@a::instance_transforms/raw_from_matrix",
            "unit": "ns",
            "value": 5.341864921173418
          },
          {
            "name": "astraweave-render@a::instance_transforms/raw_from_transform",
            "unit": "ns",
            "value": 10.303768283656927
          },
          {
            "name": "astraweave-render@a::instance_transforms/to_raw",
            "unit": "ns",
            "value": 2.495964237001582
          },
          {
            "name": "astraweave-render@a::instancing_draw_call_reduction/calculate_savings/100x10",
            "unit": "ns",
            "value": 36402.17399259063
          },
          {
            "name": "astraweave-render@a::instancing_draw_call_reduction/calculate_savings/10x100",
            "unit": "ns",
            "value": 22257.885070626373
          },
          {
            "name": "astraweave-render@a::instancing_draw_call_reduction/calculate_savings/1x1000",
            "unit": "ns",
            "value": 18501.402166520103
          },
          {
            "name": "astraweave-render@a::instancing_instance_manager/add_instances/100",
            "unit": "ns",
            "value": 2167.186137088928
          },
          {
            "name": "astraweave-render@a::instancing_instance_manager/add_instances/1000",
            "unit": "ns",
            "value": 18551.254192836434
          },
          {
            "name": "astraweave-render@a::instancing_instance_manager/add_instances/10000",
            "unit": "ns",
            "value": 181689.26490484516
          },
          {
            "name": "astraweave-render@a::instancing_pattern_generation/circle_100",
            "unit": "ns",
            "value": 1853.3192483303656
          },
          {
            "name": "astraweave-render@a::instancing_pattern_generation/grid_10x10",
            "unit": "ns",
            "value": 536.0503245230657
          },
          {
            "name": "astraweave-render@a::instancing_pattern_generation/grid_with_variations",
            "unit": "ns",
            "value": 3448.5723565858957
          },
          {
            "name": "astraweave-render@a::instancing_transformation/instance_to_raw",
            "unit": "ns",
            "value": 2.4913443026656164
          },
          {
            "name": "astraweave-render@a::integrated_full_pipeline/compress_generate_lods_instance",
            "unit": "ns",
            "value": 215550.53216752975
          },
          {
            "name": "astraweave-render@a::integrated_memory_savings/vertex_compression/1000",
            "unit": "ns",
            "value": 0.6235218401020408
          },
          {
            "name": "astraweave-render@a::integrated_memory_savings/vertex_compression/10000",
            "unit": "ns",
            "value": 0.62327052672358
          },
          {
            "name": "astraweave-render@a::integrated_memory_savings/vertex_compression/100000",
            "unit": "ns",
            "value": 0.6241195257035949
          },
          {
            "name": "astraweave-render@a::lod_generation_multi_level/generate_3_lods",
            "unit": "ns",
            "value": 446646.8021809835
          },
          {
            "name": "astraweave-render@a::lod_generation_simplification/sphere_50_percent/1089",
            "unit": "ns",
            "value": 1580864.1420128625
          },
          {
            "name": "astraweave-render@a::lod_generation_simplification/sphere_50_percent/289",
            "unit": "ns",
            "value": 207366.0862183808
          },
          {
            "name": "astraweave-render@a::lod_generation_simplification/sphere_50_percent/81",
            "unit": "ns",
            "value": 38166.733154611844
          },
          {
            "name": "astraweave-render@a::material_compile_64_nodes",
            "unit": "ns",
            "value": 7199.3290315227105
          },
          {
            "name": "astraweave-render@a::material_system/array_layout_add_10",
            "unit": "ns",
            "value": 1007.4276165457426
          },
          {
            "name": "astraweave-render@a::material_system/array_layout_lookup/10",
            "unit": "ns",
            "value": 99.60152004874485
          },
          {
            "name": "astraweave-render@a::material_system/array_layout_lookup/100",
            "unit": "ns",
            "value": 100.06522665307581
          },
          {
            "name": "astraweave-render@a::material_system/array_layout_lookup/50",
            "unit": "ns",
            "value": 98.82561784350203
          },
          {
            "name": "astraweave-render@a::material_system/batch_to_gpu/10",
            "unit": "ns",
            "value": 40.120926967642305
          },
          {
            "name": "astraweave-render@a::material_system/batch_to_gpu/100",
            "unit": "ns",
            "value": 245.91957676149264
          },
          {
            "name": "astraweave-render@a::material_system/batch_to_gpu/50",
            "unit": "ns",
            "value": 145.6530474526579
          },
          {
            "name": "astraweave-render@a::material_system/batch_to_gpu/500",
            "unit": "ns",
            "value": 1157.1401158505837
          },
          {
            "name": "astraweave-render@a::material_system/load_stats_summary",
            "unit": "ns",
            "value": 352.74024308294617
          },
          {
            "name": "astraweave-render@a::material_system/material_gpu_full_config",
            "unit": "ns",
            "value": 4.9803442136224145
          },
          {
            "name": "astraweave-render@a::material_system/material_gpu_neutral",
            "unit": "ns",
            "value": 5.917133751865114
          },
          {
            "name": "astraweave-render@a::material_system/material_layer_new",
            "unit": "ns",
            "value": 19.682639878046576
          },
          {
            "name": "astraweave-render@a::material_system/material_layer_to_gpu",
            "unit": "ns",
            "value": 8.719804415923246
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_aabb/100",
            "unit": "ns",
            "value": 221.79342928772888
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_aabb/1000",
            "unit": "ns",
            "value": 2316.0034638589295
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_aabb/10000",
            "unit": "ns",
            "value": 23317.6746728972
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_tangents/100",
            "unit": "ns",
            "value": 4341.172871067789
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_tangents/1000",
            "unit": "ns",
            "value": 41575.70298570227
          },
          {
            "name": "astraweave-render@a::mesh_operations/compute_tangents/500",
            "unit": "ns",
            "value": 20951.41838362069
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_grid/128",
            "unit": "ns",
            "value": 87859.13641791046
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_grid/16",
            "unit": "ns",
            "value": 1521.1699130891184
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_grid/32",
            "unit": "ns",
            "value": 5746.570126160438
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_grid/64",
            "unit": "ns",
            "value": 22117.36257020894
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_grid/8",
            "unit": "ns",
            "value": 477.9746823587103
          },
          {
            "name": "astraweave-render@a::mesh_operations/generate_quad",
            "unit": "ns",
            "value": 49.71572839680853
          },
          {
            "name": "astraweave-render@a::mesh_operations/mesh_memory_size",
            "unit": "ns",
            "value": 0.6220793860877769
          },
          {
            "name": "astraweave-render@a::mesh_operations/vertex_from_arrays",
            "unit": "ns",
            "value": 2.4896542800687227
          },
          {
            "name": "astraweave-render@a::mesh_operations/vertex_new",
            "unit": "ns",
            "value": 6.846513988579408
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_lookup_existing",
            "unit": "ns",
            "value": 104.7024287768351
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_lookup_missing",
            "unit": "ns",
            "value": 33.1294423004152
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_new",
            "unit": "ns",
            "value": 10.332945452223418
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_register/10",
            "unit": "ns",
            "value": 2229.286527206218
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_register/100",
            "unit": "ns",
            "value": 29246.06149909693
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_register/50",
            "unit": "ns",
            "value": 14243.01094072545
          },
          {
            "name": "astraweave-render@a::mesh_registry/registry_total_stats",
            "unit": "ns",
            "value": 114.32412361717704
          },
          {
            "name": "astraweave-render@a::overlay_params/cinematic",
            "unit": "ns",
            "value": 1.2460290459012866
          },
          {
            "name": "astraweave-render@a::overlay_params/fade_to_black",
            "unit": "ns",
            "value": 1.2455438175891345
          },
          {
            "name": "astraweave-render@a::overlay_params/interpolate_fade",
            "unit": "ns",
            "value": 1.965746840255393
          },
          {
            "name": "astraweave-render@a::overlay_params/new",
            "unit": "ns",
            "value": 2.499963802668213
          },
          {
            "name": "astraweave-render@a::overlay_params/none",
            "unit": "ns",
            "value": 0.6232452039376108
          },
          {
            "name": "astraweave-render@a::pcf_shadow/3x3_samples",
            "unit": "ns",
            "value": 0.6230445147226069
          },
          {
            "name": "astraweave-render@a::pcf_shadow/5x5_samples",
            "unit": "ns",
            "value": 0.6227153577674311
          },
          {
            "name": "astraweave-render@a::pcf_shadow/shadow_pass_1k_pixels_3x3",
            "unit": "ns",
            "value": 5624.24193608352
          },
          {
            "name": "astraweave-render@a::pcf_shadow/shadow_pass_1k_pixels_5x5",
            "unit": "ns",
            "value": 15353.217853439295
          },
          {
            "name": "astraweave-render@a::primitive_generation/cube",
            "unit": "ns",
            "value": 371.89663467685534
          },
          {
            "name": "astraweave-render@a::primitive_generation/cube_per_vertex",
            "unit": "ns",
            "value": 459.60199074911736
          },
          {
            "name": "astraweave-render@a::primitive_generation/plane",
            "unit": "ns",
            "value": 33.96018375207408
          },
          {
            "name": "astraweave-render@a::primitive_generation/sphere/16x16",
            "unit": "ns",
            "value": 3838.632548492414
          },
          {
            "name": "astraweave-render@a::primitive_generation/sphere/32x32",
            "unit": "ns",
            "value": 14037.378121892314
          },
          {
            "name": "astraweave-render@a::primitive_generation/sphere/64x64",
            "unit": "ns",
            "value": 53572.66588683351
          },
          {
            "name": "astraweave-render@a::primitive_generation/sphere/8x8",
            "unit": "ns",
            "value": 1174.638279310938
          },
          {
            "name": "astraweave-render@a::primitive_generation/sphere_16x16_per_vertex",
            "unit": "ns",
            "value": 3885.1261892682933
          },
          {
            "name": "astraweave-render@a::render_graph/clear_node_new",
            "unit": "ns",
            "value": 21.171155857883313
          },
          {
            "name": "astraweave-render@a::render_graph/full_pipeline_3_passes",
            "unit": "ns",
            "value": 69.03483174067921
          },
          {
            "name": "astraweave-render@a::render_graph/graph_context_new",
            "unit": "ns",
            "value": 5.290731079760994
          },
          {
            "name": "astraweave-render@a::render_graph/graph_execute/10",
            "unit": "ns",
            "value": 244.4725517312529
          },
          {
            "name": "astraweave-render@a::render_graph/graph_execute/20",
            "unit": "ns",
            "value": 439.32256588892534
          },
          {
            "name": "astraweave-render@a::render_graph/graph_execute/3",
            "unit": "ns",
            "value": 30.254175277198147
          },
          {
            "name": "astraweave-render@a::render_graph/graph_execute/5",
            "unit": "ns",
            "value": 73.76155866636039
          },
          {
            "name": "astraweave-render@a::render_graph/post_process_node_new",
            "unit": "ns",
            "value": 46.11301463323677
          },
          {
            "name": "astraweave-render@a::render_graph/resource_table_insert",
            "unit": "ns",
            "value": 238.0501017830427
          },
          {
            "name": "astraweave-render@a::render_graph/resource_table_lookup",
            "unit": "ns",
            "value": 138.1709682442642
          },
          {
            "name": "astraweave-render@a::render_graph/scene_node_new",
            "unit": "ns",
            "value": 41.252532221282344
          },
          {
            "name": "astraweave-render@a::rendering_frame_time/mock_full_render_pass",
            "unit": "ns",
            "value": 0.3117998925816734
          },
          {
            "name": "astraweave-render@a::scene_environment/apply_weather/None",
            "unit": "ns",
            "value": 0.6234259534598873
          },
          {
            "name": "astraweave-render@a::scene_environment/apply_weather/Rain",
            "unit": "ns",
            "value": 0.6241160305751371
          },
          {
            "name": "astraweave-render@a::scene_environment/apply_weather/Sandstorm",
            "unit": "ns",
            "value": 0.6248238684019951
          },
          {
            "name": "astraweave-render@a::scene_environment/apply_weather/Snow",
            "unit": "ns",
            "value": 0.6265993755749366
          },
          {
            "name": "astraweave-render@a::scene_environment/apply_weather/WindTrails",
            "unit": "ns",
            "value": 0.6233452114552372
          },
          {
            "name": "astraweave-render@a::scene_environment/default",
            "unit": "ns",
            "value": 12.032496546892293
          },
          {
            "name": "astraweave-render@a::scene_environment/set_biome",
            "unit": "ns",
            "value": 15.803944190368897
          },
          {
            "name": "astraweave-render@a::scene_environment/to_ubo",
            "unit": "ns",
            "value": 8.890119309583051
          },
          {
            "name": "astraweave-render@a::shader_compilation/shader_parse_and_validate",
            "unit": "ns",
            "value": 19164.413259891007
          },
          {
            "name": "astraweave-render@a::ssao_blur/bilateral_3x3",
            "unit": "ns",
            "value": 2.7791172722755717
          },
          {
            "name": "astraweave-render@a::ssao_blur/bilateral_5x5",
            "unit": "ns",
            "value": 6.187939533541319
          },
          {
            "name": "astraweave-render@a::ssao_blur/bilateral_7x7",
            "unit": "ns",
            "value": 15.804431920045136
          },
          {
            "name": "astraweave-render@a::ssao_kernel/generate/16",
            "unit": "ns",
            "value": 263.64546081894366
          },
          {
            "name": "astraweave-render@a::ssao_kernel/generate/32",
            "unit": "ns",
            "value": 541.090183953394
          },
          {
            "name": "astraweave-render@a::ssao_kernel/generate/64",
            "unit": "ns",
            "value": 1157.1665757110363
          },
          {
            "name": "astraweave-render@a::ssao_kernel/generate/8",
            "unit": "ns",
            "value": 138.4640751901744
          },
          {
            "name": "astraweave-render@a::ssao_occlusion/high_32_samples",
            "unit": "ns",
            "value": 42.15565457654574
          },
          {
            "name": "astraweave-render@a::ssao_occlusion/low_8_samples",
            "unit": "ns",
            "value": 12.055322769921709
          },
          {
            "name": "astraweave-render@a::ssao_occlusion/medium_16_samples",
            "unit": "ns",
            "value": 19.158089255884708
          },
          {
            "name": "astraweave-render@a::ssao_occlusion/ultra_64_samples",
            "unit": "ns",
            "value": 76.0242982702456
          },
          {
            "name": "astraweave-render@a::taa_blend/full_taa_pixel",
            "unit": "ns",
            "value": 106.69904739517474
          },
          {
            "name": "astraweave-render@a::taa_blend/neighborhood_clamp",
            "unit": "ns",
            "value": 107.5868162305182
          },
          {
            "name": "astraweave-render@a::taa_blend/temporal_blend",
            "unit": "ns",
            "value": 1.2456376799531788
          },
          {
            "name": "astraweave-render@a::taa_jitter/halton_sequence_16",
            "unit": "ns",
            "value": 100.39614149047404
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_allocate/10",
            "unit": "ns",
            "value": 138.0601306515483
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_allocate/100",
            "unit": "ns",
            "value": 389.2720899465613
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_allocate/50",
            "unit": "ns",
            "value": 283.0715031490832
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_allocate/500",
            "unit": "ns",
            "value": 1258.8284813687628
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_create",
            "unit": "ns",
            "value": 6.845516290851981
          },
          {
            "name": "astraweave-render@a::texture_operations/atlas_occupied_count",
            "unit": "ns",
            "value": 39.935324860336
          },
          {
            "name": "astraweave-render@a::texture_operations/calculate_mip_levels/1024x1024",
            "unit": "ns",
            "value": 1.8673032685707074
          },
          {
            "name": "astraweave-render@a::texture_operations/calculate_mip_levels/2048x2048",
            "unit": "ns",
            "value": 1.868007204943289
          },
          {
            "name": "astraweave-render@a::texture_operations/calculate_mip_levels/256x256",
            "unit": "ns",
            "value": 1.8669157470038613
          },
          {
            "name": "astraweave-render@a::texture_operations/calculate_mip_levels/4096x4096",
            "unit": "ns",
            "value": 1.8675756519455387
          },
          {
            "name": "astraweave-render@a::texture_operations/calculate_mip_levels/512x512",
            "unit": "ns",
            "value": 1.867812260392095
          },
          {
            "name": "astraweave-render@a::texture_operations/mip_size_chain",
            "unit": "ns",
            "value": 21.159771926039983
          },
          {
            "name": "astraweave-render@a::texture_operations/mipmap_levels/1",
            "unit": "ns",
            "value": 0.33232477893821916
          },
          {
            "name": "astraweave-render@a::texture_operations/mipmap_levels/3",
            "unit": "ns",
            "value": 0.33147059231491083
          },
          {
            "name": "astraweave-render@a::texture_operations/mipmap_levels/7",
            "unit": "ns",
            "value": 0.32828080150318656
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_desc_full",
            "unit": "ns",
            "value": 15.849529494343841
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_desc_new",
            "unit": "ns",
            "value": 6.535809473311813
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_memory_size/1024x1024",
            "unit": "ns",
            "value": 0.6219944730694983
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_memory_size/256x256",
            "unit": "ns",
            "value": 0.6219555866352636
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_memory_size/4096x4096",
            "unit": "ns",
            "value": 0.6227157219497323
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_usage_format",
            "unit": "ns",
            "value": 12.148192446009116
          },
          {
            "name": "astraweave-render@a::texture_operations/texture_usage_mipmaps",
            "unit": "ns",
            "value": 10.900037010161439
          },
          {
            "name": "astraweave-render@a::transition_effect/full_transition_90_frames",
            "unit": "ns",
            "value": 1554.92778753967
          },
          {
            "name": "astraweave-render@a::transition_effect/new",
            "unit": "ns",
            "value": 16.92133128307722
          },
          {
            "name": "astraweave-render@a::transition_effect/start",
            "unit": "ns",
            "value": 27.42248368070721
          },
          {
            "name": "astraweave-render@a::transition_effect/update_per_frame",
            "unit": "ns",
            "value": 12.461025681602838
          },
          {
            "name": "astraweave-render@a::vertex_compression_half_float/decode_vec2",
            "unit": "ns",
            "value": 16.82461427447656
          },
          {
            "name": "astraweave-render@a::vertex_compression_half_float/encode_vec2",
            "unit": "ns",
            "value": 35.22360812651166
          },
          {
            "name": "astraweave-render@a::vertex_compression_memory_savings/calculate_savings/1000",
            "unit": "ns",
            "value": 7.474928549143566
          },
          {
            "name": "astraweave-render@a::vertex_compression_memory_savings/calculate_savings/10000",
            "unit": "ns",
            "value": 7.475569448922556
          },
          {
            "name": "astraweave-render@a::vertex_compression_memory_savings/calculate_savings/100000",
            "unit": "ns",
            "value": 7.472479197364522
          },
          {
            "name": "astraweave-render@a::vertex_compression_memory_savings/calculate_savings/1000000",
            "unit": "ns",
            "value": 7.473501337288832
          },
          {
            "name": "astraweave-render@a::vertex_compression_octahedral/decode",
            "unit": "ns",
            "value": 24.357028648337536
          },
          {
            "name": "astraweave-render@a::vertex_compression_octahedral/encode",
            "unit": "ns",
            "value": 38.75520913043189
          },
          {
            "name": "astraweave-render@a::vertex_compression_throughput/compress_batch/100",
            "unit": "ns",
            "value": 729.6077904899389
          },
          {
            "name": "astraweave-render@a::vertex_compression_throughput/compress_batch/1000",
            "unit": "ns",
            "value": 6748.03635342665
          },
          {
            "name": "astraweave-render@a::vertex_compression_throughput/compress_batch/10000",
            "unit": "ns",
            "value": 67503.87271574125
          },
          {
            "name": "astraweave-render@a::vertex_compression_throughput/compress_batch/100000",
            "unit": "ns",
            "value": 675458.6874032202
          },
          {
            "name": "astraweave-render@b::Blend_Mode_Filter/filter_additive",
            "unit": "ns",
            "value": 1211.4550290017546
          },
          {
            "name": "astraweave-render@b::Blend_Mode_Filter/filter_all_modes",
            "unit": "ns",
            "value": 3630.620112261263
          },
          {
            "name": "astraweave-render@b::Blend_Mode_Filter/filter_alpha",
            "unit": "ns",
            "value": 1207.275124597972
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/100",
            "unit": "ns",
            "value": 1759.9973198300977
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/1000",
            "unit": "ns",
            "value": 22432.22282809612
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/10000",
            "unit": "ns",
            "value": 296157.19730769226
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/2000",
            "unit": "ns",
            "value": 49195.829351184344
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/500",
            "unit": "ns",
            "value": 10772.714871627504
          },
          {
            "name": "astraweave-render@b::Depth_Sorting/update_and_sort/5000",
            "unit": "ns",
            "value": 132163.5240710824
          },
          {
            "name": "astraweave-render@b::Full_Environment_Frame/storm_frame_5000_particles",
            "unit": "ns",
            "value": 16116.973393065835
          },
          {
            "name": "astraweave-render@b::Full_Environment_Frame/transparency_with_weather_1000",
            "unit": "ns",
            "value": 22173.310126582277
          },
          {
            "name": "astraweave-render@b::Full_Environment_Frame/typical_frame",
            "unit": "ns",
            "value": 131.29725357610724
          },
          {
            "name": "astraweave-render@b::MSAA/memory_calc_1080p",
            "unit": "ns",
            "value": 0.9365251170711743
          },
          {
            "name": "astraweave-render@b::MSAA/memory_calc_1440p",
            "unit": "ns",
            "value": 0.9344893617338983
          },
          {
            "name": "astraweave-render@b::MSAA/memory_calc_4K",
            "unit": "ns",
            "value": 0.9347113654985909
          },
          {
            "name": "astraweave-render@b::MSAA/memory_calc_720p",
            "unit": "ns",
            "value": 0.935090828689663
          },
          {
            "name": "astraweave-render@b::MSAA/mode_is_enabled",
            "unit": "ns",
            "value": 0.6231247901772452
          },
          {
            "name": "astraweave-render@b::MSAA/mode_sample_count",
            "unit": "ns",
            "value": 0.6255226113134187
          },
          {
            "name": "astraweave-render@b::MSAA/render_target_new",
            "unit": "ns",
            "value": 8.087073472438073
          },
          {
            "name": "astraweave-render@b::MSAA/render_target_set_mode",
            "unit": "ns",
            "value": 0.6227662492875439
          },
          {
            "name": "astraweave-render@b::MSAA/resize_1080p",
            "unit": "ns",
            "value": 0.622828058738835
          },
          {
            "name": "astraweave-render@b::MSAA/resize_1440p",
            "unit": "ns",
            "value": 0.6236350457584262
          },
          {
            "name": "astraweave-render@b::MSAA/resize_4K",
            "unit": "ns",
            "value": 0.622371927597661
          },
          {
            "name": "astraweave-render@b::MSAA/resize_720p",
            "unit": "ns",
            "value": 0.6230506220807767
          },
          {
            "name": "astraweave-render@b::Streaming_Stress/high_churn_100_cycles",
            "unit": "ns",
            "value": 455700.6025979685
          },
          {
            "name": "astraweave-render@b::Streaming_Stress/large_atlas_1000_textures",
            "unit": "ns",
            "value": 1128808.5051119383
          },
          {
            "name": "astraweave-render@b::Streaming_Stress/mixed_sizes_200_textures",
            "unit": "ns",
            "value": 37443.63792340052
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/evict_lru/100",
            "unit": "ns",
            "value": 24959.63602695985
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/evict_lru/1000",
            "unit": "ns",
            "value": 232893.63413980833
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/evict_lru/500",
            "unit": "ns",
            "value": 131125.40991374745
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/get_stats_500_textures",
            "unit": "ns",
            "value": 614.1797409672255
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/lru_touch_1000",
            "unit": "ns",
            "value": 96.37737498172541
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/manager_create/1024MB",
            "unit": "ns",
            "value": 13.315406664014885
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/manager_create/2048MB",
            "unit": "ns",
            "value": 13.315224539824303
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/manager_create/256MB",
            "unit": "ns",
            "value": 13.31708643777157
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/manager_create/512MB",
            "unit": "ns",
            "value": 13.342553676762227
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/priority_queue_push_pop_100",
            "unit": "ns",
            "value": 9212.956161570044
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/request_queue",
            "unit": "ns",
            "value": 3770.8010820504155
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/request_resident",
            "unit": "ns",
            "value": 98.28438669198358
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/simulate_load/128MB_budget",
            "unit": "ns",
            "value": 8752.267686753588
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/simulate_load/2048MB_budget",
            "unit": "ns",
            "value": 10316.479099020768
          },
          {
            "name": "astraweave-render@b::Texture_Streaming/simulate_load/512MB_budget",
            "unit": "ns",
            "value": 10296.861234594106
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/full_lighting_query",
            "unit": "ns",
            "value": 8.377044246200873
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/get_ambient_color",
            "unit": "ns",
            "value": 1.2462060304999814
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/get_light_color",
            "unit": "ns",
            "value": 5.913080988859077
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/get_light_direction",
            "unit": "ns",
            "value": 1.249095410739338
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/get_moon_position",
            "unit": "ns",
            "value": 1.2452955395591758
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/get_sun_position",
            "unit": "ns",
            "value": 1.2456496390276945
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/new",
            "unit": "ns",
            "value": 1.245520367708242
          },
          {
            "name": "astraweave-render@b::Time_Of_Day/time_cycle_24h",
            "unit": "ns",
            "value": 14.967964117513452
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/add_instances/100",
            "unit": "ns",
            "value": 1205.182724471831
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/add_instances/1000",
            "unit": "ns",
            "value": 21920.369783686347
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/add_instances/500",
            "unit": "ns",
            "value": 10009.404830819403
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/add_instances/5000",
            "unit": "ns",
            "value": 130234.26839309426
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/new",
            "unit": "ns",
            "value": 5.295861339366054
          },
          {
            "name": "astraweave-render@b::Transparency_Manager/with_capacity_1000",
            "unit": "ns",
            "value": 89.33707429343542
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/generate_directions/1",
            "unit": "ns",
            "value": 21.31833812231319
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/generate_directions/12",
            "unit": "ns",
            "value": 496.1326364018036
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/generate_directions/32",
            "unit": "ns",
            "value": 1042.2677514437282
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/generate_directions/4",
            "unit": "ns",
            "value": 24.21538111207599
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/generate_directions/6",
            "unit": "ns",
            "value": 23.518551399766906
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/indirect_batch/100",
            "unit": "ns",
            "value": 163345.084951473
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/indirect_batch/1000",
            "unit": "ns",
            "value": 1633418.8021810162
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/indirect_lighting/1_cones",
            "unit": "ns",
            "value": 246.50571500328107
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/indirect_lighting/4_cones",
            "unit": "ns",
            "value": 826.4266746213438
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/indirect_lighting/6_cones",
            "unit": "ns",
            "value": 1214.23413125598
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/single_cone_trace/100m",
            "unit": "ns",
            "value": 453.1090200492813
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/single_cone_trace/25m",
            "unit": "ns",
            "value": 193.81586804500282
          },
          {
            "name": "astraweave-render@b::VXGI_Cone_Tracing/single_cone_trace/50m",
            "unit": "ns",
            "value": 256.7990731490756
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/create/128_3",
            "unit": "ns",
            "value": 2600878.4475000007
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/create/256_3",
            "unit": "ns",
            "value": 22137234.526666675
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/create/64_3",
            "unit": "ns",
            "value": 54992.244878723686
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/get_voxel",
            "unit": "ns",
            "value": 2.0211589530863985
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/set_voxel",
            "unit": "ns",
            "value": 1.768218139940145
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_batch/100",
            "unit": "ns",
            "value": 3783.0148078511647
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_batch/1000",
            "unit": "ns",
            "value": 38304.57102620998
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_batch/10000",
            "unit": "ns",
            "value": 381300.6254826033
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_sample/128_3",
            "unit": "ns",
            "value": 61.92712937573886
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_sample/256_3",
            "unit": "ns",
            "value": 62.38760940216736
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/trilinear_sample/64_3",
            "unit": "ns",
            "value": 61.873724388976655
          },
          {
            "name": "astraweave-render@b::VXGI_Grid/world_to_voxel",
            "unit": "ns",
            "value": 25.644121031698866
          },
          {
            "name": "astraweave-render@b::VXGI_Voxelization/voxelize_mesh/1000_tris",
            "unit": "ns",
            "value": 485005.56523864286
          },
          {
            "name": "astraweave-render@b::VXGI_Voxelization/voxelize_mesh/100_tris",
            "unit": "ns",
            "value": 422431.2127774011
          },
          {
            "name": "astraweave-render@b::VXGI_Voxelization/voxelize_mesh/500_tris",
            "unit": "ns",
            "value": 443453.7270191999
          },
          {
            "name": "astraweave-render@b::VXGI_Voxelization/voxelize_triangle_large",
            "unit": "ns",
            "value": 1150.277835806473
          },
          {
            "name": "astraweave-render@b::VXGI_Voxelization/voxelize_triangle_small",
            "unit": "ns",
            "value": 35.59586726467448
          },
          {
            "name": "astraweave-render@b::Weather_Particles/new_1000",
            "unit": "ns",
            "value": 87.07046529592527
          },
          {
            "name": "astraweave-render@b::Weather_Particles/new_10000",
            "unit": "ns",
            "value": 88.11269813485987
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_rain/100",
            "unit": "ns",
            "value": 3163.166704753199
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_rain/1000",
            "unit": "ns",
            "value": 37279.10652680653
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_rain/500",
            "unit": "ns",
            "value": 17868.371129792908
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_rain/5000",
            "unit": "ns",
            "value": 662890.0122448981
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_snow/100",
            "unit": "ns",
            "value": 3335.849052239064
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_snow/1000",
            "unit": "ns",
            "value": 38164.96699669967
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_snow/500",
            "unit": "ns",
            "value": 18472.030504879083
          },
          {
            "name": "astraweave-render@b::Weather_Particles/spawn_snow/5000",
            "unit": "ns",
            "value": 679621.1020979022
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_rain/100",
            "unit": "ns",
            "value": 2.1824580178538167
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_rain/1000",
            "unit": "ns",
            "value": 2.1834737462697476
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_rain/500",
            "unit": "ns",
            "value": 2.1823569891477037
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_rain/5000",
            "unit": "ns",
            "value": 2.207530046602894
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_snow/100",
            "unit": "ns",
            "value": 2.49594842626441
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_snow/1000",
            "unit": "ns",
            "value": 65.87919087136929
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_snow/500",
            "unit": "ns",
            "value": 15.27966793222644
          },
          {
            "name": "astraweave-render@b::Weather_Particles/update_snow/5000",
            "unit": "ns",
            "value": 1333.4741163475703
          },
          {
            "name": "astraweave-render@b::Weather_System/all_weather_types_query",
            "unit": "ns",
            "value": 194.2077265097571
          },
          {
            "name": "astraweave-render@b::Weather_System/get_all_intensities",
            "unit": "ns",
            "value": 6.5393621258009915
          },
          {
            "name": "astraweave-render@b::Weather_System/get_light_attenuation",
            "unit": "ns",
            "value": 0.6341345983508418
          },
          {
            "name": "astraweave-render@b::Weather_System/get_terrain_modifier",
            "unit": "ns",
            "value": 1.2479574833568643
          },
          {
            "name": "astraweave-render@b::Weather_System/new",
            "unit": "ns",
            "value": 36.12536352999863
          },
          {
            "name": "astraweave-render@b::Weather_System/set_weather_instant",
            "unit": "ns",
            "value": 28.00429450018412
          },
          {
            "name": "astraweave-render@b::Weather_System/set_weather_transition",
            "unit": "ns",
            "value": 28.03689803891313
          },
          {
            "name": "astraweave-render@b::Weather_System/update",
            "unit": "ns",
            "value": 100.60643371101209
          },
          {
            "name": "astraweave-render@b::animation_blending/crossfade_20_joints",
            "unit": "ns",
            "value": 201.08644534491356
          },
          {
            "name": "astraweave-render@b::animation_blending/crossfade_stress/100",
            "unit": "ns",
            "value": 926.8190418106701
          },
          {
            "name": "astraweave-render@b::animation_blending/crossfade_stress/200",
            "unit": "ns",
            "value": 1813.330569919796
          },
          {
            "name": "astraweave-render@b::animation_blending/crossfade_stress/50",
            "unit": "ns",
            "value": 489.12077317276174
          },
          {
            "name": "astraweave-render@b::animation_full_frame/characters/1",
            "unit": "ns",
            "value": 1923.0524226704074
          },
          {
            "name": "astraweave-render@b::animation_full_frame/characters/10",
            "unit": "ns",
            "value": 18426.753786663743
          },
          {
            "name": "astraweave-render@b::animation_full_frame/characters/100",
            "unit": "ns",
            "value": 185515.1583827083
          },
          {
            "name": "astraweave-render@b::animation_full_frame/characters/50",
            "unit": "ns",
            "value": 91787.27959334778
          },
          {
            "name": "astraweave-render@b::animation_hierarchy/forward_kinematics/100",
            "unit": "ns",
            "value": 2201.246751759084
          },
          {
            "name": "astraweave-render@b::animation_hierarchy/forward_kinematics/20",
            "unit": "ns",
            "value": 462.84168407476363
          },
          {
            "name": "astraweave-render@b::animation_hierarchy/forward_kinematics/200",
            "unit": "ns",
            "value": 4381.033928475375
          },
          {
            "name": "astraweave-render@b::animation_hierarchy/forward_kinematics/50",
            "unit": "ns",
            "value": 1116.1037490277922
          },
          {
            "name": "astraweave-render@b::animation_joint_palette/humanoid_20_joints",
            "unit": "ns",
            "value": 3946.7874702962795
          },
          {
            "name": "astraweave-render@b::animation_joint_palette/stress/100",
            "unit": "ns",
            "value": 6952.095018811681
          },
          {
            "name": "astraweave-render@b::animation_joint_palette/stress/200",
            "unit": "ns",
            "value": 9582.645178575798
          },
          {
            "name": "astraweave-render@b::animation_joint_palette/stress/50",
            "unit": "ns",
            "value": 5065.151282456985
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/binary/16",
            "unit": "ns",
            "value": 8.481491875696186
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/binary/256",
            "unit": "ns",
            "value": 14.867059163613703
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/binary/4",
            "unit": "ns",
            "value": 7.790995376876009
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/binary/64",
            "unit": "ns",
            "value": 11.8579927285973
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/linear/16",
            "unit": "ns",
            "value": 12.477223869683007
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/linear/256",
            "unit": "ns",
            "value": 157.27425240588457
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/linear/4",
            "unit": "ns",
            "value": 7.786275221028933
          },
          {
            "name": "astraweave-render@b::animation_keyframe_search/linear/64",
            "unit": "ns",
            "value": 37.73842985028198
          },
          {
            "name": "astraweave-render@b::animation_matrix/multiply",
            "unit": "ns",
            "value": 6.33097621763754
          },
          {
            "name": "astraweave-render@b::animation_matrix/quat_to_rotation",
            "unit": "ns",
            "value": 1.8702068794942113
          },
          {
            "name": "astraweave-render@b::animation_sample/humanoid_20_joints",
            "unit": "ns",
            "value": 606.5148878340469
          },
          {
            "name": "astraweave-render@b::animation_sample/stress/100",
            "unit": "ns",
            "value": 2909.6714026686464
          },
          {
            "name": "astraweave-render@b::animation_sample/stress/200",
            "unit": "ns",
            "value": 5788.159448502377
          },
          {
            "name": "astraweave-render@b::animation_sample/stress/50",
            "unit": "ns",
            "value": 1472.964628693147
          },
          {
            "name": "astraweave-render@b::animation_transform/lerp",
            "unit": "ns",
            "value": 32.81412601295606
          },
          {
            "name": "astraweave-render@b::animation_transform/quat_slerp",
            "unit": "ns",
            "value": 29.028269562020533
          },
          {
            "name": "astraweave-render@b::animation_transform/to_matrix",
            "unit": "ns",
            "value": 8.41165379500315
          },
          {
            "name": "astraweave-render@b::animation_transform/vec3_lerp",
            "unit": "ns",
            "value": 1.5591784890536642
          },
          {
            "name": "astraweave-render@b::cpu_light_culling/bin_lights_cpu/100",
            "unit": "ns",
            "value": 425181.65088505344
          },
          {
            "name": "astraweave-render@b::cpu_light_culling/bin_lights_cpu/1000",
            "unit": "ns",
            "value": 3878305.50153846
          },
          {
            "name": "astraweave-render@b::cpu_light_culling/bin_lights_cpu/2000",
            "unit": "ns",
            "value": 7520078.564285713
          },
          {
            "name": "astraweave-render@b::cpu_light_culling/bin_lights_cpu/250",
            "unit": "ns",
            "value": 889946.5002601413
          },
          {
            "name": "astraweave-render@b::cpu_light_culling/bin_lights_cpu/500",
            "unit": "ns",
            "value": 1909376.0044385227
          },
          {
            "name": "astraweave-render@b::gpu_particles_culling/frustum_cull/10000",
            "unit": "ns",
            "value": 12354.911613988976
          },
          {
            "name": "astraweave-render@b::gpu_particles_culling/frustum_cull/100000",
            "unit": "ns",
            "value": 130074.95105057683
          },
          {
            "name": "astraweave-render@b::gpu_particles_culling/frustum_cull/50000",
            "unit": "ns",
            "value": 64225.93088071487
          },
          {
            "name": "astraweave-render@b::gpu_particles_emission/emit_particles/100",
            "unit": "ns",
            "value": 331.6233338137169
          },
          {
            "name": "astraweave-render@b::gpu_particles_emission/emit_particles/1000",
            "unit": "ns",
            "value": 3305.42249683493
          },
          {
            "name": "astraweave-render@b::gpu_particles_emission/emit_particles/500",
            "unit": "ns",
            "value": 1673.8347487717206
          },
          {
            "name": "astraweave-render@b::gpu_particles_emission/emit_particles/5000",
            "unit": "ns",
            "value": 16678.466518907797
          },
          {
            "name": "astraweave-render@b::gpu_particles_full_frame/update_cull_sort/10000",
            "unit": "ns",
            "value": 1292657.0095535861
          },
          {
            "name": "astraweave-render@b::gpu_particles_full_frame/update_cull_sort/100000",
            "unit": "ns",
            "value": 16931086.053333327
          },
          {
            "name": "astraweave-render@b::gpu_particles_full_frame/update_cull_sort/50000",
            "unit": "ns",
            "value": 7826338.725714287
          },
          {
            "name": "astraweave-render@b::gpu_particles_sorting/depth_sort/1000",
            "unit": "ns",
            "value": 2012.0500551715747
          },
          {
            "name": "astraweave-render@b::gpu_particles_sorting/depth_sort/10000",
            "unit": "ns",
            "value": 21117.168704547676
          },
          {
            "name": "astraweave-render@b::gpu_particles_sorting/depth_sort/5000",
            "unit": "ns",
            "value": 10234.050278612931
          },
          {
            "name": "astraweave-render@b::gpu_particles_sorting/depth_sort/50000",
            "unit": "ns",
            "value": 137296.25946829154
          },
          {
            "name": "astraweave-render@b::gpu_particles_update/update_particles/1000",
            "unit": "ns",
            "value": 1803.484479027024
          },
          {
            "name": "astraweave-render@b::gpu_particles_update/update_particles/10000",
            "unit": "ns",
            "value": 19066.760481379133
          },
          {
            "name": "astraweave-render@b::gpu_particles_update/update_particles/100000",
            "unit": "ns",
            "value": 189000.57133822973
          },
          {
            "name": "astraweave-render@b::gpu_particles_update/update_particles/50000",
            "unit": "ns",
            "value": 94275.34552691673
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/generate_full_lut/128x128",
            "unit": "ns",
            "value": 29009820.65
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/generate_full_lut/64x64",
            "unit": "ns",
            "value": 7300265.25
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/generate_row/128",
            "unit": "ns",
            "value": 229412.48865740746
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/generate_row/256",
            "unit": "ns",
            "value": 455271.9840182647
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/generate_row/64",
            "unit": "ns",
            "value": 116139.13169014084
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/integrate_single_sample/1024",
            "unit": "ns",
            "value": 0.6591400564842
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/integrate_single_sample/256",
            "unit": "ns",
            "value": 0.6227250975877887
          },
          {
            "name": "astraweave-render@b::ibl_deferred_brdf_lut/integrate_single_sample/64",
            "unit": "ns",
            "value": 0.6233959460437836
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/batch_sample_1k_directions",
            "unit": "ns",
            "value": 8982.328135193911
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/bilinear_sample/1024x1024",
            "unit": "ns",
            "value": 9.981960327346702
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/bilinear_sample/128x128",
            "unit": "ns",
            "value": 9.98174190898335
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/bilinear_sample/256x256",
            "unit": "ns",
            "value": 9.975593760136231
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/bilinear_sample/512x512",
            "unit": "ns",
            "value": 9.980821973448961
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/bilinear_sample/64x64",
            "unit": "ns",
            "value": 9.981346624704452
          },
          {
            "name": "astraweave-render@b::ibl_deferred_cubemap_sampling/direction_to_uv_1k",
            "unit": "ns",
            "value": 2490.7266450616503
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/accumulate_lights/1000",
            "unit": "ns",
            "value": 24097.642188258127
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/accumulate_lights/128",
            "unit": "ns",
            "value": 3088.9980024731285
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/accumulate_lights/32",
            "unit": "ns",
            "value": 770.9003836600182
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/accumulate_lights/512",
            "unit": "ns",
            "value": 12331.065566856858
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/accumulate_lights/8",
            "unit": "ns",
            "value": 193.53397337233326
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/process_1k_pixels/32lights",
            "unit": "ns",
            "value": 787455.0015748033
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/process_1k_pixels/8lights",
            "unit": "ns",
            "value": 195777.85719921105
          },
          {
            "name": "astraweave-render@b::ibl_deferred_deferred_lighting/single_point_light",
            "unit": "ns",
            "value": 23.962402497601232
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/gbuffer_fill_simulation/1280x720",
            "unit": "ns",
            "value": 573823.8419540229
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/gbuffer_fill_simulation/1920x1080",
            "unit": "ns",
            "value": 1291257.9324675326
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/gbuffer_fill_simulation/2560x1440",
            "unit": "ns",
            "value": 2294995.3863636362
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/gbuffer_fill_simulation/3840x2160",
            "unit": "ns",
            "value": 5168577.705
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/pack_gbuffer_pixel_1k",
            "unit": "ns",
            "value": 7848.281361206415
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/pack_normal_octahedral_1k",
            "unit": "ns",
            "value": 4244.436474005984
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/unpack_gbuffer_pixel_1k",
            "unit": "ns",
            "value": 9551.561234662577
          },
          {
            "name": "astraweave-render@b::ibl_deferred_gbuffer_operations/unpack_normal_octahedral_1k",
            "unit": "ns",
            "value": 5639.10988700565
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/hammersley_sequence/1024",
            "unit": "ns",
            "value": 9598.151303181296
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/hammersley_sequence/256",
            "unit": "ns",
            "value": 1918.8160898919753
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/hammersley_sequence/4096",
            "unit": "ns",
            "value": 46035.66662062615
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/hammersley_sequence/64",
            "unit": "ns",
            "value": 361.75067391376103
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n1024_r0.25",
            "unit": "ns",
            "value": 24057.910725786474
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n1024_r0.50",
            "unit": "ns",
            "value": 24044.454033041788
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n1024_r0.75",
            "unit": "ns",
            "value": 24060.82342168675
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n256_r0.25",
            "unit": "ns",
            "value": 5490.569801044793
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n256_r0.50",
            "unit": "ns",
            "value": 5474.120446964643
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n256_r0.75",
            "unit": "ns",
            "value": 5485.638860009672
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n64_r0.25",
            "unit": "ns",
            "value": 1182.9252859663625
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n64_r0.50",
            "unit": "ns",
            "value": 1183.2989167311466
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/prefilter_kernel/n64_r0.75",
            "unit": "ns",
            "value": 1183.5439406951502
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/single_sample/r0.10",
            "unit": "ns",
            "value": 20.303086164820723
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/single_sample/r0.25",
            "unit": "ns",
            "value": 20.2983713535301
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/single_sample/r0.50",
            "unit": "ns",
            "value": 20.299261284352248
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/single_sample/r0.75",
            "unit": "ns",
            "value": 20.292001028796058
          },
          {
            "name": "astraweave-render@b::ibl_deferred_ggx_importance_sampling/single_sample/r1.00",
            "unit": "ns",
            "value": 20.28215802827051
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/evaluate_basis/SH16",
            "unit": "ns",
            "value": 33.93766375154614
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/evaluate_basis/SH25",
            "unit": "ns",
            "value": 37.92482775763935
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/evaluate_basis/SH9",
            "unit": "ns",
            "value": 31.564576830135497
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/generate_coefficients/SH16",
            "unit": "ns",
            "value": 30927223.6
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/generate_coefficients/SH25",
            "unit": "ns",
            "value": 36098424.94
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/generate_coefficients/SH9",
            "unit": "ns",
            "value": 22630498.08
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/reconstruct_irradiance_sh16",
            "unit": "ns",
            "value": 39.34767320456997
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/reconstruct_irradiance_sh25",
            "unit": "ns",
            "value": 50.69332077270675
          },
          {
            "name": "astraweave-render@b::ibl_deferred_spherical_harmonics/reconstruct_irradiance_sh9",
            "unit": "ns",
            "value": 32.12746069811675
          },
          {
            "name": "astraweave-render@b::water_full_frame/animate_full/128",
            "unit": "ns",
            "value": 1713140.575488683
          },
          {
            "name": "astraweave-render@b::water_full_frame/animate_full/256",
            "unit": "ns",
            "value": 6759580.3975
          },
          {
            "name": "astraweave-render@b::water_full_frame/animate_full/64",
            "unit": "ns",
            "value": 430244.01064944617
          },
          {
            "name": "astraweave-render@b::water_gerstner_combined/displacement_4waves",
            "unit": "ns",
            "value": 49.979629434523794
          },
          {
            "name": "astraweave-render@b::water_gerstner_combined/foam_calculation",
            "unit": "ns",
            "value": 49.17365569459029
          },
          {
            "name": "astraweave-render@b::water_gerstner_combined/fresnel",
            "unit": "ns",
            "value": 1.252782498194822
          },
          {
            "name": "astraweave-render@b::water_gerstner_combined/normal_4waves",
            "unit": "ns",
            "value": 63.87476537760544
          },
          {
            "name": "astraweave-render@b::water_gerstner_single/displacement",
            "unit": "ns",
            "value": 18.47150803331167
          },
          {
            "name": "astraweave-render@b::water_gerstner_single/normal",
            "unit": "ns",
            "value": 9.920842269328991
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_indices/128",
            "unit": "ns",
            "value": 63415.48077560235
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_indices/256",
            "unit": "ns",
            "value": 252675.04247378942
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_indices/32",
            "unit": "ns",
            "value": 3887.635885432188
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_indices/64",
            "unit": "ns",
            "value": 15804.30563722823
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_vertices/128",
            "unit": "ns",
            "value": 32348.19194083232
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_vertices/256",
            "unit": "ns",
            "value": 127379.55982055866
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_vertices/32",
            "unit": "ns",
            "value": 2300.274024648657
          },
          {
            "name": "astraweave-render@b::water_grid_generation/generate_vertices/64",
            "unit": "ns",
            "value": 8454.300633516277
          },
          {
            "name": "astraweave-render@b::water_surface/animate_grid/128",
            "unit": "ns",
            "value": 789220.19216573
          },
          {
            "name": "astraweave-render@b::water_surface/animate_grid/256",
            "unit": "ns",
            "value": 3048933.565294117
          },
          {
            "name": "astraweave-render@b::water_surface/animate_grid/32",
            "unit": "ns",
            "value": 46685.206950106796
          },
          {
            "name": "astraweave-render@b::water_surface/animate_grid/64",
            "unit": "ns",
            "value": 184759.17495713316
          },
          {
            "name": "astraweave-render@b::water_surface/calculate_normals/128",
            "unit": "ns",
            "value": 905707.7683762542
          },
          {
            "name": "astraweave-render@b::water_surface/calculate_normals/256",
            "unit": "ns",
            "value": 3562795.950666666
          },
          {
            "name": "astraweave-render@b::water_surface/calculate_normals/32",
            "unit": "ns",
            "value": 57157.45028229999
          },
          {
            "name": "astraweave-render@b::water_surface/calculate_normals/64",
            "unit": "ns",
            "value": 222150.86492062648
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/build_cascades/2",
            "unit": "ns",
            "value": 213.74101744955505
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/build_cascades/4",
            "unit": "ns",
            "value": 382.7900052979052
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/build_cascades/6",
            "unit": "ns",
            "value": 586.5715779722716
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/build_cascades/8",
            "unit": "ns",
            "value": 739.1388766136661
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/calculate_splits_4_cascades",
            "unit": "ns",
            "value": 49.717542444560735
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/calculate_splits_8_cascades",
            "unit": "ns",
            "value": 83.59748057570454
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/select_cascade_far",
            "unit": "ns",
            "value": 3.1340405193031176
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/select_cascade_mid",
            "unit": "ns",
            "value": 1.8756478324880883
          },
          {
            "name": "astraweave-render@c::Cascade_Splitting/select_cascade_near",
            "unit": "ns",
            "value": 1.2497150754173456
          },
          {
            "name": "astraweave-render@c::Cluster_Grid/create/16x9x24",
            "unit": "ns",
            "value": 8184.556966139055
          },
          {
            "name": "astraweave-render@c::Cluster_Grid/create/32x18x48",
            "unit": "ns",
            "value": 63738.98489037508
          },
          {
            "name": "astraweave-render@c::Cluster_Grid/create/64x36x96",
            "unit": "ns",
            "value": 517083.96953817166
          },
          {
            "name": "astraweave-render@c::Cluster_Grid/single_cluster_access",
            "unit": "ns",
            "value": 0.9390418397266599
          },
          {
            "name": "astraweave-render@c::Cull_Stats/visibility/10%",
            "unit": "ns",
            "value": 52026.76974214862
          },
          {
            "name": "astraweave-render@c::Cull_Stats/visibility/50%",
            "unit": "ns",
            "value": 167850.66994634093
          },
          {
            "name": "astraweave-render@c::Cull_Stats/visibility/90%",
            "unit": "ns",
            "value": 283452.88722238387
          },
          {
            "name": "astraweave-render@c::Full_Shadow_Pass/2_cascades/10000",
            "unit": "ns",
            "value": 272071.7961428854
          },
          {
            "name": "astraweave-render@c::Full_Shadow_Pass/2_cascades/100000",
            "unit": "ns",
            "value": 2785038.184444445
          },
          {
            "name": "astraweave-render@c::Full_Shadow_Pass/4_cascades/10000",
            "unit": "ns",
            "value": 284211.8400680748
          },
          {
            "name": "astraweave-render@c::Full_Shadow_Pass/4_cascades/100000",
            "unit": "ns",
            "value": 2895531.2877777778
          },
          {
            "name": "astraweave-render@c::GpuCamera/from_matrix_1080p",
            "unit": "ns",
            "value": 54.828617919450316
          },
          {
            "name": "astraweave-render@c::GpuCamera/from_matrix_4K",
            "unit": "ns",
            "value": 55.26068443213862
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/build/1920x1080",
            "unit": "ns",
            "value": 2622091.305263158
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/build/2560x1440",
            "unit": "ns",
            "value": 4713655.07090909
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/build/3840x2160",
            "unit": "ns",
            "value": 11283779.057999996
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/downsample_single_level",
            "unit": "ns",
            "value": 1817005.8544749033
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/sample_mip0",
            "unit": "ns",
            "value": 2.8170631409356606
          },
          {
            "name": "astraweave-render@c::HiZ_Pyramid/sample_mip4",
            "unit": "ns",
            "value": 2.6190434056781338
          },
          {
            "name": "astraweave-render@c::Light_Binning/cpu_full_pipeline/100",
            "unit": "ns",
            "value": 1945048.4422245421
          },
          {
            "name": "astraweave-render@c::Light_Binning/cpu_full_pipeline/1000",
            "unit": "ns",
            "value": 16704007.558168722
          },
          {
            "name": "astraweave-render@c::Light_Binning/cpu_full_pipeline/2000",
            "unit": "ns",
            "value": 32857770.32222222
          },
          {
            "name": "astraweave-render@c::Light_Binning/cpu_full_pipeline/500",
            "unit": "ns",
            "value": 8631973.735226016
          },
          {
            "name": "astraweave-render@c::Light_Binning/cpu_full_pipeline/5000",
            "unit": "ns",
            "value": 81155664.14444445
          },
          {
            "name": "astraweave-render@c::Light_Binning/density/high_density",
            "unit": "ns",
            "value": 132520879.38333334
          },
          {
            "name": "astraweave-render@c::Light_Binning/density/low_density",
            "unit": "ns",
            "value": 2809511.1783420015
          },
          {
            "name": "astraweave-render@c::Light_Binning/density/standard",
            "unit": "ns",
            "value": 16765420.998430096
          },
          {
            "name": "astraweave-render@c::Light_Intersection/cone_sphere_hit",
            "unit": "ns",
            "value": 5.983485049382901
          },
          {
            "name": "astraweave-render@c::Light_Intersection/cone_sphere_miss",
            "unit": "ns",
            "value": 5.982391988768793
          },
          {
            "name": "astraweave-render@c::Light_Intersection/sphere_aabb_hit",
            "unit": "ns",
            "value": 2.9192425613370347
          },
          {
            "name": "astraweave-render@c::Light_Intersection/sphere_aabb_miss",
            "unit": "ns",
            "value": 3.5485783962636606
          },
          {
            "name": "astraweave-render@c::Light_Intersection/sphere_sphere_hit",
            "unit": "ns",
            "value": 3.3312180660064197
          },
          {
            "name": "astraweave-render@c::Light_Intersection/sphere_sphere_miss",
            "unit": "ns",
            "value": 3.3273709505648985
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/intersection_density/16x9x24",
            "unit": "ns",
            "value": 7467997.931982281
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/intersection_density/32x18x48",
            "unit": "ns",
            "value": 59760020.53000001
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/intersection_density/64x36x96",
            "unit": "ns",
            "value": 472751631.65
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/workgroup_calc/16x9x24",
            "unit": "ns",
            "value": 5.919187915049616
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/workgroup_calc/32x18x48",
            "unit": "ns",
            "value": 5.9251949527973995
          },
          {
            "name": "astraweave-render@c::MegaLights_Scaling/workgroup_calc/64x36x96",
            "unit": "ns",
            "value": 5.92407101856711
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/backface_single",
            "unit": "ns",
            "value": 10.924466559588042
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/frustum_single_culled",
            "unit": "ns",
            "value": 2.507119009280332
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/frustum_single_visible",
            "unit": "ns",
            "value": 5.457208974049701
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/full_cull/1000",
            "unit": "ns",
            "value": 13018.921837085125
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/full_cull/10000",
            "unit": "ns",
            "value": 171040.3025302222
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/full_cull/5000",
            "unit": "ns",
            "value": 65031.55752899792
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/full_cull/50000",
            "unit": "ns",
            "value": 1531430.5402921105
          },
          {
            "name": "astraweave-render@c::Meshlet_Culling/occlusion_single",
            "unit": "ns",
            "value": 24.648028622082776
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/batch_5x5/1000",
            "unit": "ns",
            "value": 38294.87391350065
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/batch_5x5/10000",
            "unit": "ns",
            "value": 471328.0355448353
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/batch_5x5/100000",
            "unit": "ns",
            "value": 4791783.731818183
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/kernel/3x3",
            "unit": "ns",
            "value": 35.3368539115867
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/kernel/5x5",
            "unit": "ns",
            "value": 82.61735008369732
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/kernel/7x7",
            "unit": "ns",
            "value": 152.93719556374845
          },
          {
            "name": "astraweave-render@c::PCF_Sampling/kernel/9x9",
            "unit": "ns",
            "value": 251.38315662407288
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/blelloch/1024",
            "unit": "ns",
            "value": 1835.9554780219787
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/blelloch/16384",
            "unit": "ns",
            "value": 51125.674325451626
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/blelloch/4096",
            "unit": "ns",
            "value": 12104.427917416298
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/blelloch/65536",
            "unit": "ns",
            "value": 204319.50861099712
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/sequential/1024",
            "unit": "ns",
            "value": 759.3402919050652
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/sequential/16384",
            "unit": "ns",
            "value": 11183.902112436183
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/sequential/4096",
            "unit": "ns",
            "value": 2849.32314406108
          },
          {
            "name": "astraweave-render@c::Prefix_Sum/sequential/65536",
            "unit": "ns",
            "value": 44679.75606360943
          },
          {
            "name": "astraweave-render@c::Residency_Manager/create",
            "unit": "ns",
            "value": 8.116732545750155
          },
          {
            "name": "astraweave-render@c::Residency_Manager/evict_by_priority",
            "unit": "ns",
            "value": 2393953.7405263167
          },
          {
            "name": "astraweave-render@c::Residency_Manager/evict_lru",
            "unit": "ns",
            "value": 7118.124432943075
          },
          {
            "name": "astraweave-render@c::Residency_Manager/hot_reload",
            "unit": "ns",
            "value": 33773.28172263925
          },
          {
            "name": "astraweave-render@c::Residency_Manager/load_new_asset",
            "unit": "ns",
            "value": 184.58356280067557
          },
          {
            "name": "astraweave-render@c::Residency_Manager/touch_asset",
            "unit": "ns",
            "value": 88.73959366285403
          },
          {
            "name": "astraweave-render@c::Residency_Stress/frame_simulation_100_frames",
            "unit": "ns",
            "value": 220256.9258729872
          },
          {
            "name": "astraweave-render@c::Residency_Stress/high_churn/100",
            "unit": "ns",
            "value": 20428.0650098572
          },
          {
            "name": "astraweave-render@c::Residency_Stress/high_churn/1000",
            "unit": "ns",
            "value": 188754.74603953006
          },
          {
            "name": "astraweave-render@c::Residency_Stress/high_churn/500",
            "unit": "ns",
            "value": 99098.4766811653
          },
          {
            "name": "astraweave-render@c::Residency_Stress/large_assets_pressure",
            "unit": "ns",
            "value": 19025.296570479262
          },
          {
            "name": "astraweave-render@c::Shadow_Matrix/calculate_projection",
            "unit": "ns",
            "value": 63.50160214945043
          },
          {
            "name": "astraweave-render@c::Shadow_Matrix/to_gpu_cascade",
            "unit": "ns",
            "value": 3.757769978191234
          },
          {
            "name": "astraweave-render@c::VSM_Sampling/batch/1000",
            "unit": "ns",
            "value": 2129.221068709502
          },
          {
            "name": "astraweave-render@c::VSM_Sampling/batch/10000",
            "unit": "ns",
            "value": 21285.664903633642
          },
          {
            "name": "astraweave-render@c::VSM_Sampling/batch/100000",
            "unit": "ns",
            "value": 212702.80818235257
          },
          {
            "name": "astraweave-render@c::VSM_Sampling/calculate_moments",
            "unit": "ns",
            "value": 0.9132292037417646
          },
          {
            "name": "astraweave-render@c::VSM_Sampling/chebyshev_bound",
            "unit": "ns",
            "value": 2.075331693113992
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/color_grading_apply",
            "unit": "ns",
            "value": 3.4745597020049437
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/color_grading_config_default",
            "unit": "ns",
            "value": 8.108909717143042
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_calculate_coc",
            "unit": "ns",
            "value": 1.7785502856236188
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_coc_at_depth/10m",
            "unit": "ns",
            "value": 1.2522036544814656
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_coc_at_depth/15m",
            "unit": "ns",
            "value": 1.8802333712698722
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_coc_at_depth/20m",
            "unit": "ns",
            "value": 1.8726810740532926
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_coc_at_depth/50m",
            "unit": "ns",
            "value": 1.876455112484655
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_coc_at_depth/5m",
            "unit": "ns",
            "value": 2.00246816167092
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/dof_config_default",
            "unit": "ns",
            "value": 6.2405190193279285
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/full_post_process_setup",
            "unit": "ns",
            "value": 1.2525862537290773
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/halton_sequence",
            "unit": "ns",
            "value": 16.927669688495776
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/motion_blur_config_default",
            "unit": "ns",
            "value": 2.501997882123735
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/post_process_config_default",
            "unit": "ns",
            "value": 10.291703766963543
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/taa_config_default",
            "unit": "ns",
            "value": 8.109108198404485
          },
          {
            "name": "astraweave-render@c::advanced_post_processing/taa_halton_jitter",
            "unit": "ns",
            "value": 15.187296473052275
          },
          {
            "name": "astraweave-render@c::combined_scenarios/depth_overlay_frame",
            "unit": "ns",
            "value": 8.740711779720638
          },
          {
            "name": "astraweave-render@c::combined_scenarios/full_render_frame_setup",
            "unit": "ns",
            "value": 637.4937298536651
          },
          {
            "name": "astraweave-render@c::combined_scenarios/memory_allocation_batch",
            "unit": "ns",
            "value": 3.755029705867441
          },
          {
            "name": "astraweave-render@c::combined_scenarios/multi_skeleton_batch_10",
            "unit": "ns",
            "value": 67354.99146174864
          },
          {
            "name": "astraweave-render@c::combined_scenarios/resolution_change",
            "unit": "ns",
            "value": 6.548042284101581
          },
          {
            "name": "astraweave-render@c::combined_scenarios/skeletal_frame_64_joints",
            "unit": "ns",
            "value": 7233.262199488492
          },
          {
            "name": "astraweave-render@c::combined_scenarios/terrain_full_setup",
            "unit": "ns",
            "value": 21.94298420906124
          },
          {
            "name": "astraweave-render@c::decals/atlas_uv_lookup",
            "unit": "ns",
            "value": 13.130218958835005
          },
          {
            "name": "astraweave-render@c::decals/fade_update/100",
            "unit": "ns",
            "value": 81.536387642488
          },
          {
            "name": "astraweave-render@c::decals/fade_update/1000",
            "unit": "ns",
            "value": 818.2143416232228
          },
          {
            "name": "astraweave-render@c::decals/fade_update/500",
            "unit": "ns",
            "value": 404.17708909177105
          },
          {
            "name": "astraweave-render@c::decals/full_update/200",
            "unit": "ns",
            "value": 978.7734396382858
          },
          {
            "name": "astraweave-render@c::decals/full_update/50",
            "unit": "ns",
            "value": 267.4885434082538
          },
          {
            "name": "astraweave-render@c::decals/full_update/500",
            "unit": "ns",
            "value": 2452.7192433912496
          },
          {
            "name": "astraweave-render@c::decals/gpu_decal_size",
            "unit": "ns",
            "value": 0.623518780499485
          },
          {
            "name": "astraweave-render@c::decals/to_gpu_batch/10",
            "unit": "ns",
            "value": 107.42752040032863
          },
          {
            "name": "astraweave-render@c::decals/to_gpu_batch/100",
            "unit": "ns",
            "value": 656.54132348728
          },
          {
            "name": "astraweave-render@c::decals/to_gpu_batch/1000",
            "unit": "ns",
            "value": 6183.201213603668
          },
          {
            "name": "astraweave-render@c::decals/to_gpu_batch/500",
            "unit": "ns",
            "value": 3143.173159239173
          },
          {
            "name": "astraweave-render@c::decals/to_gpu_single",
            "unit": "ns",
            "value": 10.433733197188225
          },
          {
            "name": "astraweave-render@c::depth_buffer/buffer_creation",
            "unit": "ns",
            "value": 7.167069601936535
          },
          {
            "name": "astraweave-render@c::depth_buffer/buffer_memory_size",
            "unit": "ns",
            "value": 0.6229213055073948
          },
          {
            "name": "astraweave-render@c::depth_buffer/buffer_resize",
            "unit": "ns",
            "value": 1.2468594059366136
          },
          {
            "name": "astraweave-render@c::depth_buffer/desc_aspect_ratio",
            "unit": "ns",
            "value": 0.6364603930763634
          },
          {
            "name": "astraweave-render@c::depth_buffer/desc_creation",
            "unit": "ns",
            "value": 3.741063987076347
          },
          {
            "name": "astraweave-render@c::depth_buffer/desc_memory_size",
            "unit": "ns",
            "value": 0.6258751426608997
          },
          {
            "name": "astraweave-render@c::depth_buffer/desc_with_msaa",
            "unit": "ns",
            "value": 2.4958819719504524
          },
          {
            "name": "astraweave-render@c::depth_buffer/format_bytes_per_pixel",
            "unit": "ns",
            "value": 0.6253264288543352
          },
          {
            "name": "astraweave-render@c::depth_buffer/format_has_stencil",
            "unit": "ns",
            "value": 0.6252905881784155
          },
          {
            "name": "astraweave-render@c::depth_buffer/memory_size_resolution/1280x720",
            "unit": "ns",
            "value": 0.6241359193599818
          },
          {
            "name": "astraweave-render@c::depth_buffer/memory_size_resolution/1920x1080",
            "unit": "ns",
            "value": 0.6244420259831568
          },
          {
            "name": "astraweave-render@c::depth_buffer/memory_size_resolution/2560x1440",
            "unit": "ns",
            "value": 0.6241322319734283
          },
          {
            "name": "astraweave-render@c::depth_buffer/memory_size_resolution/3840x2160",
            "unit": "ns",
            "value": 0.6248139601198428
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/allocation_throughput/10",
            "unit": "ns",
            "value": 17.64494392116384
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/allocation_throughput/100",
            "unit": "ns",
            "value": 114.56038531080773
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/allocation_throughput/1000",
            "unit": "ns",
            "value": 1052.226935991272
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/can_allocate_check",
            "unit": "ns",
            "value": 0.9367388660891682
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/category_all_iteration",
            "unit": "ns",
            "value": 0.624258455174085
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/category_remaining",
            "unit": "ns",
            "value": 0.6232453247842494
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/category_report",
            "unit": "ns",
            "value": 19.68566889932255
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/free",
            "unit": "ns",
            "value": 1.2387667015539416
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/manager_creation_custom_4gb",
            "unit": "ns",
            "value": 14.65041136481805
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/manager_creation_default",
            "unit": "ns",
            "value": 14.456967476031664
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/pressure_level",
            "unit": "ns",
            "value": 0.6386594770616438
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/try_allocate",
            "unit": "ns",
            "value": 1.998357581110302
          },
          {
            "name": "astraweave-render@c::gpu_memory_budget/usage_ratio",
            "unit": "ns",
            "value": 0.6400035607336759
          },
          {
            "name": "astraweave-render@c::overlay_effects/animation_sequence_10_frames",
            "unit": "ns",
            "value": 0.6263797038542991
          },
          {
            "name": "astraweave-render@c::overlay_effects/as_bytes",
            "unit": "ns",
            "value": 0.6249126299814305
          },
          {
            "name": "astraweave-render@c::overlay_effects/is_active",
            "unit": "ns",
            "value": 0.6247967991944131
          },
          {
            "name": "astraweave-render@c::overlay_effects/params_default",
            "unit": "ns",
            "value": 0.6249350272766068
          },
          {
            "name": "astraweave-render@c::overlay_effects/params_lerp",
            "unit": "ns",
            "value": 1.653348092176363
          },
          {
            "name": "astraweave-render@c::overlay_effects/set_fade",
            "unit": "ns",
            "value": 2.5030329033862344
          },
          {
            "name": "astraweave-render@c::overlay_effects/set_letterbox",
            "unit": "ns",
            "value": 2.193594865269461
          },
          {
            "name": "astraweave-render@c::overlay_effects/set_vignette",
            "unit": "ns",
            "value": 6.558260959901716
          },
          {
            "name": "astraweave-render@c::overlay_effects/size_constant",
            "unit": "ns",
            "value": 0.6247962980269117
          },
          {
            "name": "astraweave-render@c::skinning_gpu/get_matrix",
            "unit": "ns",
            "value": 11.334744940616073
          },
          {
            "name": "astraweave-render@c::skinning_gpu/handle_creation",
            "unit": "ns",
            "value": 0.9345133839027568
          },
          {
            "name": "astraweave-render@c::skinning_gpu/manager_allocate",
            "unit": "ns",
            "value": 6724.092118551043
          },
          {
            "name": "astraweave-render@c::skinning_gpu/manager_creation",
            "unit": "ns",
            "value": 9.056281769160467
          },
          {
            "name": "astraweave-render@c::skinning_gpu/manager_free",
            "unit": "ns",
            "value": 965.0792453411683
          },
          {
            "name": "astraweave-render@c::skinning_gpu/manager_get",
            "unit": "ns",
            "value": 12.752495869638157
          },
          {
            "name": "astraweave-render@c::skinning_gpu/manager_upload",
            "unit": "ns",
            "value": 607.0512438325496
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_as_bytes",
            "unit": "ns",
            "value": 0.6246363528427075
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_default",
            "unit": "ns",
            "value": 6252.509742304479
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_from_identity/128",
            "unit": "ns",
            "value": 6722.543489355985
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_from_identity/16",
            "unit": "ns",
            "value": 6554.568988216708
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_from_identity/256",
            "unit": "ns",
            "value": 6919.112048689911
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_from_identity/64",
            "unit": "ns",
            "value": 6646.571446590984
          },
          {
            "name": "astraweave-render@c::skinning_gpu/palette_size_constant",
            "unit": "ns",
            "value": 0.6249400939282553
          },
          {
            "name": "astraweave-render@c::skinning_gpu/set_matrix",
            "unit": "ns",
            "value": 3.122789140809743
          },
          {
            "name": "astraweave-render@c::ssr/binary_refine/16",
            "unit": "ns",
            "value": 278.6054953426177
          },
          {
            "name": "astraweave-render@c::ssr/binary_refine/4",
            "unit": "ns",
            "value": 56.74759886860902
          },
          {
            "name": "astraweave-render@c::ssr/binary_refine/8",
            "unit": "ns",
            "value": 130.66662882247024
          },
          {
            "name": "astraweave-render@c::ssr/cone_trace_r/0",
            "unit": "ns",
            "value": 185.86843875259106
          },
          {
            "name": "astraweave-render@c::ssr/cone_trace_r/100",
            "unit": "ns",
            "value": 1749.0536198434136
          },
          {
            "name": "astraweave-render@c::ssr/cone_trace_r/25",
            "unit": "ns",
            "value": 574.593484293791
          },
          {
            "name": "astraweave-render@c::ssr/cone_trace_r/50",
            "unit": "ns",
            "value": 968.1506678982347
          },
          {
            "name": "astraweave-render@c::ssr/fullscreen/1280x720",
            "unit": "ns",
            "value": 1457032.2740903443
          },
          {
            "name": "astraweave-render@c::ssr/fullscreen/320x180",
            "unit": "ns",
            "value": 89209.82099784425
          },
          {
            "name": "astraweave-render@c::ssr/fullscreen/640x360",
            "unit": "ns",
            "value": 359637.34017521655
          },
          {
            "name": "astraweave-render@c::ssr/ray_march/128",
            "unit": "ns",
            "value": 599.2719436198062
          },
          {
            "name": "astraweave-render@c::ssr/ray_march/16",
            "unit": "ns",
            "value": 81.27417998471594
          },
          {
            "name": "astraweave-render@c::ssr/ray_march/32",
            "unit": "ns",
            "value": 155.79423291622564
          },
          {
            "name": "astraweave-render@c::ssr/ray_march/64",
            "unit": "ns",
            "value": 307.8590671469519
          },
          {
            "name": "astraweave-render@c::terrain_materials/compute_layer_weight",
            "unit": "ns",
            "value": 1.4242171431609307
          },
          {
            "name": "astraweave-render@c::terrain_materials/get_layer",
            "unit": "ns",
            "value": 0.9362653575010764
          },
          {
            "name": "astraweave-render@c::terrain_materials/layer_gpu_default",
            "unit": "ns",
            "value": 2.5008359508149645
          },
          {
            "name": "astraweave-render@c::terrain_materials/layer_gpu_new",
            "unit": "ns",
            "value": 7.192237397354328
          },
          {
            "name": "astraweave-render@c::terrain_materials/layer_set_height_blend",
            "unit": "ns",
            "value": 8.514789361680792
          },
          {
            "name": "astraweave-render@c::terrain_materials/layer_set_material",
            "unit": "ns",
            "value": 3.7633628343098486
          },
          {
            "name": "astraweave-render@c::terrain_materials/normalize_weights",
            "unit": "ns",
            "value": 10.571905248796954
          },
          {
            "name": "astraweave-render@c::terrain_materials/set_splat_map",
            "unit": "ns",
            "value": 19.348825497247105
          },
          {
            "name": "astraweave-render@c::terrain_materials/set_triplanar",
            "unit": "ns",
            "value": 20.918809775777984
          },
          {
            "name": "astraweave-render@c::terrain_materials/size_constant_layer",
            "unit": "ns",
            "value": 0.6238026805738539
          },
          {
            "name": "astraweave-render@c::terrain_materials/size_constant_material",
            "unit": "ns",
            "value": 0.6230911377940535
          },
          {
            "name": "astraweave-render@c::terrain_materials/terrain_material_default",
            "unit": "ns",
            "value": 20.16506222952483
          },
          {
            "name": "astraweave-render@c::terrain_materials/terrain_material_with_layers",
            "unit": "ns",
            "value": 24.48445076433979
          },
          {
            "name": "astraweave-render@c::weather/full_frame/1000",
            "unit": "ns",
            "value": 11731.414446440642
          },
          {
            "name": "astraweave-render@c::weather/full_frame/2000",
            "unit": "ns",
            "value": 23217.894470305495
          },
          {
            "name": "astraweave-render@c::weather/full_frame/500",
            "unit": "ns",
            "value": 5966.438487694926
          },
          {
            "name": "astraweave-render@c::weather/spawn_rain",
            "unit": "ns",
            "value": 8.754464505901677
          },
          {
            "name": "astraweave-render@c::weather/spawn_rain_batch/100",
            "unit": "ns",
            "value": 553.1932895501582
          },
          {
            "name": "astraweave-render@c::weather/spawn_rain_batch/1000",
            "unit": "ns",
            "value": 5184.793749518126
          },
          {
            "name": "astraweave-render@c::weather/spawn_rain_batch/500",
            "unit": "ns",
            "value": 2616.723165904951
          },
          {
            "name": "astraweave-render@c::weather/spawn_rain_batch/5000",
            "unit": "ns",
            "value": 25313.270929411636
          },
          {
            "name": "astraweave-render@c::weather/spawn_wind",
            "unit": "ns",
            "value": 8.73460117165226
          },
          {
            "name": "astraweave-render@c::weather/to_instance_batch/100",
            "unit": "ns",
            "value": 422.7265864060934
          },
          {
            "name": "astraweave-render@c::weather/to_instance_batch/1000",
            "unit": "ns",
            "value": 3884.6049673638136
          },
          {
            "name": "astraweave-render@c::weather/to_instance_batch/500",
            "unit": "ns",
            "value": 1954.6868256681835
          },
          {
            "name": "astraweave-render@c::weather/to_instance_batch/5000",
            "unit": "ns",
            "value": 19490.703975269706
          },
          {
            "name": "astraweave-render@c::weather/to_instance_single",
            "unit": "ns",
            "value": 13.52514704719352
          },
          {
            "name": "astraweave-render@c::weather/update_batch/100",
            "unit": "ns",
            "value": 166.39148148482434
          },
          {
            "name": "astraweave-render@c::weather/update_batch/1000",
            "unit": "ns",
            "value": 1704.1362640197706
          },
          {
            "name": "astraweave-render@c::weather/update_batch/500",
            "unit": "ns",
            "value": 814.1344689184582
          },
          {
            "name": "astraweave-render@c::weather/update_batch/5000",
            "unit": "ns",
            "value": 8635.783843288578
          },
          {
            "name": "astraweave-render@c::weather/update_single",
            "unit": "ns",
            "value": 3.7442409986153313
          },
          {
            "name": "astraweave-dialogue::dialogue_clone/clone_graph/10",
            "unit": "ns",
            "value": 1447.0635553017567
          },
          {
            "name": "astraweave-dialogue::dialogue_clone/clone_graph/100",
            "unit": "ns",
            "value": 28665.762008914968
          },
          {
            "name": "astraweave-dialogue::dialogue_clone/clone_graph/200",
            "unit": "ns",
            "value": 57370.56596602805
          },
          {
            "name": "astraweave-dialogue::dialogue_clone/clone_graph/50",
            "unit": "ns",
            "value": 14192.067286679749
          },
          {
            "name": "astraweave-dialogue::dialogue_clone/clone_node",
            "unit": "ns",
            "value": 117.3136864097686
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/branching_construction/d3_b3",
            "unit": "ns",
            "value": 12596.82526632939
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/branching_construction/d4_b2",
            "unit": "ns",
            "value": 9714.650837731511
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/branching_construction/d5_b2",
            "unit": "ns",
            "value": 22874.888870682422
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/complex_construction/100",
            "unit": "ns",
            "value": 60115.227377073876
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/complex_construction/25",
            "unit": "ns",
            "value": 14305.794330451381
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/complex_construction/50",
            "unit": "ns",
            "value": 29506.792114760974
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/linear_construction/10",
            "unit": "ns",
            "value": 2181.8909376885263
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/linear_construction/100",
            "unit": "ns",
            "value": 31626.92847057924
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/linear_construction/50",
            "unit": "ns",
            "value": 15219.046591843551
          },
          {
            "name": "astraweave-dialogue::dialogue_construction/linear_construction/500",
            "unit": "ns",
            "value": 151528.82844482272
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_first/10",
            "unit": "ns",
            "value": 3.637681470055284
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_first/100",
            "unit": "ns",
            "value": 3.431708706750278
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_first/1000",
            "unit": "ns",
            "value": 3.4351966527493865
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_first/50",
            "unit": "ns",
            "value": 3.4303423995718902
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_first/500",
            "unit": "ns",
            "value": 3.7398639077058475
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_last/10",
            "unit": "ns",
            "value": 35.26824784946019
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_last/100",
            "unit": "ns",
            "value": 356.8475220667656
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_last/1000",
            "unit": "ns",
            "value": 3698.361805504298
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_last/50",
            "unit": "ns",
            "value": 152.21099618664755
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_last/500",
            "unit": "ns",
            "value": 1668.951793765399
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_middle/10",
            "unit": "ns",
            "value": 19.540152921622795
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_middle/100",
            "unit": "ns",
            "value": 143.49892183772312
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_middle/1000",
            "unit": "ns",
            "value": 1428.3758483587321
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_middle/50",
            "unit": "ns",
            "value": 57.261954092500815
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_middle/500",
            "unit": "ns",
            "value": 533.7173075859878
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_nonexistent/10",
            "unit": "ns",
            "value": 4.92259305105221
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_nonexistent/100",
            "unit": "ns",
            "value": 39.59453110349854
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_nonexistent/1000",
            "unit": "ns",
            "value": 664.709687383409
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_nonexistent/50",
            "unit": "ns",
            "value": 18.734647276673627
          },
          {
            "name": "astraweave-dialogue::dialogue_node_lookup/lookup_nonexistent/500",
            "unit": "ns",
            "value": 165.6539064125868
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/from_json/10",
            "unit": "ns",
            "value": 4706.398021082094
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/from_json/100",
            "unit": "ns",
            "value": 50223.70616695305
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/from_json/50",
            "unit": "ns",
            "value": 24963.651056182407
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/roundtrip/10",
            "unit": "ns",
            "value": 8177.0143238275605
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/roundtrip/100",
            "unit": "ns",
            "value": 89211.29946409211
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/roundtrip/50",
            "unit": "ns",
            "value": 44506.124431889
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json/10",
            "unit": "ns",
            "value": 2825.1728619150354
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json/100",
            "unit": "ns",
            "value": 27128.26966807156
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json/200",
            "unit": "ns",
            "value": 54196.617465392614
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json/50",
            "unit": "ns",
            "value": 13778.011739713289
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json_pretty/10",
            "unit": "ns",
            "value": 4835.4765054832715
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json_pretty/100",
            "unit": "ns",
            "value": 45776.37807657023
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json_pretty/200",
            "unit": "ns",
            "value": 91146.03708790883
          },
          {
            "name": "astraweave-dialogue::dialogue_serialization/to_json_pretty/50",
            "unit": "ns",
            "value": 24160.073539884314
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/count_responses/100",
            "unit": "ns",
            "value": 21.767957347672347
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/count_responses/200",
            "unit": "ns",
            "value": 49.93587546336545
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/count_responses/50",
            "unit": "ns",
            "value": 10.18984572091779
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/follow_linear_path_100",
            "unit": "ns",
            "value": 18556.044721276037
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/iterate_all_nodes/100",
            "unit": "ns",
            "value": 63.09202694594579
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/iterate_all_nodes/1000",
            "unit": "ns",
            "value": 630.1157193541009
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/iterate_all_nodes/50",
            "unit": "ns",
            "value": 31.787930305070322
          },
          {
            "name": "astraweave-dialogue::dialogue_traversal/iterate_all_nodes/500",
            "unit": "ns",
            "value": 317.6056082157549
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/branching_graph/d3_b3",
            "unit": "ns",
            "value": 1835.0414199493698
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/branching_graph/d3_b4",
            "unit": "ns",
            "value": 4096.03461281764
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/branching_graph/d4_b2",
            "unit": "ns",
            "value": 1400.5523010461675
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/branching_graph/d5_b2",
            "unit": "ns",
            "value": 2927.6011123337767
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/complex_graph/100",
            "unit": "ns",
            "value": 8434.14749436836
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/complex_graph/200",
            "unit": "ns",
            "value": 16688.314822451877
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/complex_graph/25",
            "unit": "ns",
            "value": 1910.4699005282964
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/complex_graph/50",
            "unit": "ns",
            "value": 4019.7723185210725
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/cyclic_graph/10",
            "unit": "ns",
            "value": 501.1819919792278
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/cyclic_graph/100",
            "unit": "ns",
            "value": 4897.640473039387
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/cyclic_graph/200",
            "unit": "ns",
            "value": 9485.966439532373
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/cyclic_graph/50",
            "unit": "ns",
            "value": 2389.1344072017628
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/linear_graph/10",
            "unit": "ns",
            "value": 418.4940709958089
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/linear_graph/100",
            "unit": "ns",
            "value": 4713.004565786771
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/linear_graph/50",
            "unit": "ns",
            "value": 2326.122810288601
          },
          {
            "name": "astraweave-dialogue::dialogue_validation/linear_graph/500",
            "unit": "ns",
            "value": 22953.84582702822
          },
          {
            "name": "astraweave-llm::cache_capacity_impact/cap_10",
            "unit": "ns",
            "value": 67598.18977621412
          },
          {
            "name": "astraweave-llm::cache_capacity_impact/cap_100",
            "unit": "ns",
            "value": 67407.62561586674
          },
          {
            "name": "astraweave-llm::cache_capacity_impact/cap_200",
            "unit": "ns",
            "value": 67472.28127036395
          },
          {
            "name": "astraweave-llm::cache_capacity_impact/cap_50",
            "unit": "ns",
            "value": 67718.2974060602
          },
          {
            "name": "astraweave-llm::cache_capacity_impact/cap_500",
            "unit": "ns",
            "value": 68113.17947502866
          },
          {
            "name": "astraweave-llm::cache_hit_latency",
            "unit": "ns",
            "value": 64.43464441994685
          },
          {
            "name": "astraweave-llm::cache_hit_rate_validation",
            "unit": "ns",
            "value": 67956.52064398897
          },
          {
            "name": "astraweave-llm::cache_key_generation",
            "unit": "ns",
            "value": 4336.085189771443
          },
          {
            "name": "astraweave-llm::cache_miss_latency/100ms_llm",
            "unit": "ns",
            "value": 101144075.74
          },
          {
            "name": "astraweave-llm::cache_miss_latency/10ms_llm",
            "unit": "ns",
            "value": 11158289.564000001
          },
          {
            "name": "astraweave-llm::cache_miss_latency/200ms_llm",
            "unit": "ns",
            "value": 201170671.42
          },
          {
            "name": "astraweave-llm::cache_miss_latency/50ms_llm",
            "unit": "ns",
            "value": 51180217.84
          },
          {
            "name": "astraweave-llm::cache_stress_1000_requests",
            "unit": "ns",
            "value": 80944.12424969733
          },
          {
            "name": "astraweave-llm::circuit_breaker_chaos/10%_failures",
            "unit": "ns",
            "value": 1305.141920862899
          },
          {
            "name": "astraweave-llm::circuit_breaker_chaos/30%_failures",
            "unit": "ns",
            "value": 1424.2369296590716
          },
          {
            "name": "astraweave-llm::circuit_breaker_chaos/50%_failures",
            "unit": "ns",
            "value": 1399.0209071065667
          },
          {
            "name": "astraweave-llm::circuit_breaker_chaos/70%_failures",
            "unit": "ns",
            "value": 1556.7621524360334
          },
          {
            "name": "astraweave-llm::circuit_breaker_opening",
            "unit": "ns",
            "value": 62.42743593778863
          },
          {
            "name": "astraweave-llm::circuit_breaker_per_model_isolation",
            "unit": "ns",
            "value": 360.49962313386874
          },
          {
            "name": "astraweave-llm::circuit_breaker_recovery",
            "unit": "ns",
            "value": 16177339.0925
          },
          {
            "name": "astraweave-llm::circuit_breaker_state_check",
            "unit": "ns",
            "value": 5.031042897167969
          },
          {
            "name": "astraweave-llm::concurrent_cache_access",
            "unit": "ns",
            "value": 147428.8933060397
          },
          {
            "name": "astraweave-llm::end_to_end_plan_generation/cache_hit",
            "unit": "ns",
            "value": 2282580.1926988047
          },
          {
            "name": "astraweave-llm::end_to_end_plan_generation/cache_miss_fast",
            "unit": "ns",
            "value": 52312716.56
          },
          {
            "name": "astraweave-llm::end_to_end_plan_generation/cache_miss_slow",
            "unit": "ns",
            "value": 202387305.36
          },
          {
            "name": "astraweave-llm::lru_eviction_overhead",
            "unit": "ns",
            "value": 68480.26291138463
          },
          {
            "name": "astraweave-llm::prompt_normalization",
            "unit": "ns",
            "value": 910.1949140892808
          },
          {
            "name": "astraweave-llm::retry_backoff_calculation",
            "unit": "ns",
            "value": 46.38513058547867
          },
          {
            "name": "astraweave-llm::retry_execution_3_attempts",
            "unit": "ns",
            "value": 152357867.16
          },
          {
            "name": "astraweave-llm::retry_with_circuit_breaker",
            "unit": "ns",
            "value": 22.087545333196932
          },
          {
            "name": "astraweave-llm::telemetry_record_request",
            "unit": "ns",
            "value": 6.723522377903459
          },
          {
            "name": "astraweave-steam::steam_adversarial_achievements/filter_achievements_5000",
            "unit": "ns",
            "value": 21098.379102870167
          },
          {
            "name": "astraweave-steam::steam_adversarial_achievements/load_achievements_500",
            "unit": "ns",
            "value": 83896.617588297
          },
          {
            "name": "astraweave-steam::steam_adversarial_achievements/progress_update_10000",
            "unit": "ns",
            "value": 630168.0184725695
          },
          {
            "name": "astraweave-steam::steam_adversarial_achievements/unlock_achievements_1000",
            "unit": "ns",
            "value": 62310.78132978992
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/conflict_resolution_1000",
            "unit": "ns",
            "value": 4654.8751811962165
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/create_save/1024",
            "unit": "ns",
            "value": 48.58939510225151
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/create_save/16384",
            "unit": "ns",
            "value": 231.47923120803742
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/create_save/262144",
            "unit": "ns",
            "value": 2953.3718512184587
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/create_save/65536",
            "unit": "ns",
            "value": 787.953730976171
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/file_management_5000",
            "unit": "ns",
            "value": 2386126.772272727
          },
          {
            "name": "astraweave-steam::steam_adversarial_cloud_saves/serialize_save_data_1000",
            "unit": "ns",
            "value": 286198.49420662655
          },
          {
            "name": "astraweave-steam::steam_adversarial_leaderboards/query_leaderboard_5000",
            "unit": "ns",
            "value": 6014010.282222221
          },
          {
            "name": "astraweave-steam::steam_adversarial_leaderboards/score_deduplication_5000",
            "unit": "ns",
            "value": 127208.78613541114
          },
          {
            "name": "astraweave-steam::steam_adversarial_leaderboards/submit_scores_10000",
            "unit": "ns",
            "value": 414877.91104270174
          },
          {
            "name": "astraweave-steam::steam_adversarial_leaderboards/time_leaderboard_processing_2000",
            "unit": "ns",
            "value": 108060.40468899261
          },
          {
            "name": "astraweave-steam::steam_adversarial_platform_api/api_call_batching_10000",
            "unit": "ns",
            "value": 1242099.0229188313
          },
          {
            "name": "astraweave-steam::steam_adversarial_platform_api/callback_processing_10000",
            "unit": "ns",
            "value": 923893.752170835
          },
          {
            "name": "astraweave-steam::steam_adversarial_platform_api/rate_limiting_20000",
            "unit": "ns",
            "value": 1468227.1012502026
          },
          {
            "name": "astraweave-steam::steam_adversarial_platform_api/session_management_5000",
            "unit": "ns",
            "value": 113616.47470148194
          },
          {
            "name": "astraweave-steam::steam_adversarial_statistics/init_stats_200",
            "unit": "ns",
            "value": 12666.277191765357
          },
          {
            "name": "astraweave-steam::steam_adversarial_statistics/stat_aggregation_10000",
            "unit": "ns",
            "value": 19566.880004018873
          },
          {
            "name": "astraweave-steam::steam_adversarial_statistics/stat_updates_50000",
            "unit": "ns",
            "value": 3192992.716875
          },
          {
            "name": "astraweave-steam::steam_adversarial_statistics/stat_validation_20000",
            "unit": "ns",
            "value": 1510870.314217812
          },
          {
            "name": "astraweave-steam::steam_adversarial_workshop/download_queue_1000",
            "unit": "ns",
            "value": 17903.06268717299
          },
          {
            "name": "astraweave-steam::steam_adversarial_workshop/filter_sort_workshop_5000",
            "unit": "ns",
            "value": 497056.306995556
          },
          {
            "name": "astraweave-steam::steam_adversarial_workshop/load_workshop_items_1000",
            "unit": "ns",
            "value": 349485.35103904543
          },
          {
            "name": "astraweave-steam::steam_adversarial_workshop/subscription_management_2000",
            "unit": "ns",
            "value": 49676.04704339357
          },
          {
            "name": "astraweave-embeddings::batch_operations/batch_insert/10",
            "unit": "ns",
            "value": 5181.3523788081575
          },
          {
            "name": "astraweave-embeddings::batch_operations/batch_insert/100",
            "unit": "ns",
            "value": 63009.67697501593
          },
          {
            "name": "astraweave-embeddings::batch_operations/batch_insert/50",
            "unit": "ns",
            "value": 30601.991938753585
          },
          {
            "name": "astraweave-embeddings::batch_operations/batch_insert/500",
            "unit": "ns",
            "value": 340154.37164004485
          },
          {
            "name": "astraweave-embeddings::batch_operations/get_all_ids/100",
            "unit": "ns",
            "value": 4996.869410848505
          },
          {
            "name": "astraweave-embeddings::batch_operations/get_all_ids/1000",
            "unit": "ns",
            "value": 44757.68373056032
          },
          {
            "name": "astraweave-embeddings::batch_operations/get_all_ids/2000",
            "unit": "ns",
            "value": 95684.3964969719
          },
          {
            "name": "astraweave-embeddings::batch_operations/get_all_ids/500",
            "unit": "ns",
            "value": 24330.012347880533
          },
          {
            "name": "astraweave-embeddings::batch_operations/prune_vectors/1000_to_500",
            "unit": "ns",
            "value": 152153.025
          },
          {
            "name": "astraweave-embeddings::batch_operations/prune_vectors/100_to_50",
            "unit": "ns",
            "value": 22756.299357644883
          },
          {
            "name": "astraweave-embeddings::batch_operations/prune_vectors/500_to_250",
            "unit": "ns",
            "value": 91955.14166666668
          },
          {
            "name": "astraweave-embeddings::distance_calculations/cosine_distance/1024",
            "unit": "ns",
            "value": 2799.4585919962415
          },
          {
            "name": "astraweave-embeddings::distance_calculations/cosine_distance/128",
            "unit": "ns",
            "value": 271.2130143215495
          },
          {
            "name": "astraweave-embeddings::distance_calculations/cosine_distance/384",
            "unit": "ns",
            "value": 988.5102178315934
          },
          {
            "name": "astraweave-embeddings::distance_calculations/cosine_distance/768",
            "unit": "ns",
            "value": 2073.7101267940643
          },
          {
            "name": "astraweave-embeddings::distance_calculations/dot_product/1024",
            "unit": "ns",
            "value": 930.5874338868631
          },
          {
            "name": "astraweave-embeddings::distance_calculations/dot_product/128",
            "unit": "ns",
            "value": 93.83055725397102
          },
          {
            "name": "astraweave-embeddings::distance_calculations/dot_product/384",
            "unit": "ns",
            "value": 333.4858841100616
          },
          {
            "name": "astraweave-embeddings::distance_calculations/dot_product/768",
            "unit": "ns",
            "value": 691.7953766152408
          },
          {
            "name": "astraweave-embeddings::distance_calculations/euclidean_distance/1024",
            "unit": "ns",
            "value": 938.7951417172932
          },
          {
            "name": "astraweave-embeddings::distance_calculations/euclidean_distance/128",
            "unit": "ns",
            "value": 102.51186480190206
          },
          {
            "name": "astraweave-embeddings::distance_calculations/euclidean_distance/384",
            "unit": "ns",
            "value": 341.01394941062733
          },
          {
            "name": "astraweave-embeddings::distance_calculations/euclidean_distance/768",
            "unit": "ns",
            "value": 699.6968650724514
          },
          {
            "name": "astraweave-embeddings::distance_calculations/manhattan_distance/1024",
            "unit": "ns",
            "value": 937.1766769484585
          },
          {
            "name": "astraweave-embeddings::distance_calculations/manhattan_distance/128",
            "unit": "ns",
            "value": 96.9529519755102
          },
          {
            "name": "astraweave-embeddings::distance_calculations/manhattan_distance/384",
            "unit": "ns",
            "value": 339.6269213933616
          },
          {
            "name": "astraweave-embeddings::distance_calculations/manhattan_distance/768",
            "unit": "ns",
            "value": 698.0852881686131
          },
          {
            "name": "astraweave-embeddings::embedding_client/batch_embed/1",
            "unit": "ns",
            "value": 333.65955792127147
          },
          {
            "name": "astraweave-embeddings::embedding_client/batch_embed/10",
            "unit": "ns",
            "value": 2582.042681741826
          },
          {
            "name": "astraweave-embeddings::embedding_client/batch_embed/20",
            "unit": "ns",
            "value": 5222.964625036939
          },
          {
            "name": "astraweave-embeddings::embedding_client/batch_embed/5",
            "unit": "ns",
            "value": 1297.9705906559568
          },
          {
            "name": "astraweave-embeddings::embedding_client/batch_embed/50",
            "unit": "ns",
            "value": 13152.93270693228
          },
          {
            "name": "astraweave-embeddings::embedding_client/model_info",
            "unit": "ns",
            "value": 46.83474863829844
          },
          {
            "name": "astraweave-embeddings::embedding_client/single_embed",
            "unit": "ns",
            "value": 280.13938548627664
          },
          {
            "name": "astraweave-embeddings::memory_operations/create_combat_memory",
            "unit": "ns",
            "value": 652.9455491419521
          },
          {
            "name": "astraweave-embeddings::memory_operations/create_exploration_memory",
            "unit": "ns",
            "value": 547.2499269952066
          },
          {
            "name": "astraweave-embeddings::memory_operations/create_memory",
            "unit": "ns",
            "value": 566.8766329325434
          },
          {
            "name": "astraweave-embeddings::memory_operations/create_social_memory",
            "unit": "ns",
            "value": 567.8170888908307
          },
          {
            "name": "astraweave-embeddings::serialization/from_json/10",
            "unit": "ns",
            "value": 130895.60934151529
          },
          {
            "name": "astraweave-embeddings::serialization/from_json/100",
            "unit": "ns",
            "value": 1361982.628307301
          },
          {
            "name": "astraweave-embeddings::serialization/from_json/50",
            "unit": "ns",
            "value": 678527.5292083218
          },
          {
            "name": "astraweave-embeddings::serialization/to_json/10",
            "unit": "ns",
            "value": 123060.4898629758
          },
          {
            "name": "astraweave-embeddings::serialization/to_json/100",
            "unit": "ns",
            "value": 1388728.4880147052
          },
          {
            "name": "astraweave-embeddings::serialization/to_json/50",
            "unit": "ns",
            "value": 686702.6539995307
          },
          {
            "name": "astraweave-embeddings::serialization/to_json/500",
            "unit": "ns",
            "value": 6937734.95875
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/chunk_text/10000_1000",
            "unit": "ns",
            "value": 474.67977138297005
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/chunk_text/1000_200",
            "unit": "ns",
            "value": 183.3277329379399
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/chunk_text/5000_500",
            "unit": "ns",
            "value": 452.23308299808735
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/extract_keyphrases",
            "unit": "ns",
            "value": 1385.104634133857
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/preprocess/100",
            "unit": "ns",
            "value": 193.56718257223326
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/preprocess/1000",
            "unit": "ns",
            "value": 1496.740833300628
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/preprocess/500",
            "unit": "ns",
            "value": 850.9299137053961
          },
          {
            "name": "astraweave-embeddings::text_preprocessing/preprocess/5000",
            "unit": "ns",
            "value": 6922.7339503348885
          },
          {
            "name": "astraweave-embeddings::vector_search/brute_force_k10/100",
            "unit": "ns",
            "value": 126773.74774266242
          },
          {
            "name": "astraweave-embeddings::vector_search/brute_force_k10/1000",
            "unit": "ns",
            "value": 1344685.1416272055
          },
          {
            "name": "astraweave-embeddings::vector_search/brute_force_k10/2000",
            "unit": "ns",
            "value": 2787467.175789474
          },
          {
            "name": "astraweave-embeddings::vector_search/brute_force_k10/500",
            "unit": "ns",
            "value": 669423.1108028023
          },
          {
            "name": "astraweave-embeddings::vector_search/search_k_variation/1",
            "unit": "ns",
            "value": 1346478.1631247923
          },
          {
            "name": "astraweave-embeddings::vector_search/search_k_variation/10",
            "unit": "ns",
            "value": 1347906.4561564315
          },
          {
            "name": "astraweave-embeddings::vector_search/search_k_variation/20",
            "unit": "ns",
            "value": 1349343.2766408422
          },
          {
            "name": "astraweave-embeddings::vector_search/search_k_variation/5",
            "unit": "ns",
            "value": 1348344.3829822915
          },
          {
            "name": "astraweave-embeddings::vector_search/search_k_variation/50",
            "unit": "ns",
            "value": 1349694.9664151228
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/get_by_id/100",
            "unit": "ns",
            "value": 174.25524053749004
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/get_by_id/1000",
            "unit": "ns",
            "value": 204.34450849859775
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/get_by_id/5000",
            "unit": "ns",
            "value": 213.53865506838633
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/insert_single",
            "unit": "ns",
            "value": 8110.3816123507995
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/insert_with_metadata",
            "unit": "ns",
            "value": 8134.536775895474
          },
          {
            "name": "astraweave-embeddings::vector_store_operations/remove_single",
            "unit": "ns",
            "value": 11921.790265006102
          },
          {
            "name": "astraweave-ecs::archetype_transitions/add_remove_cycle",
            "unit": "ns",
            "value": 561649.1253825744
          },
          {
            "name": "astraweave-ecs::archetype_transitions/multi_component_transitions",
            "unit": "ns",
            "value": 2231618.6782608684
          },
          {
            "name": "astraweave-ecs::component_add/multiple_components/100",
            "unit": "ns",
            "value": 96578.20425904162
          },
          {
            "name": "astraweave-ecs::component_add/multiple_components/1000",
            "unit": "ns",
            "value": 937053.6565920541
          },
          {
            "name": "astraweave-ecs::component_add/multiple_components/10000",
            "unit": "ns",
            "value": 9450457.813333336
          },
          {
            "name": "astraweave-ecs::component_add/single_component/100",
            "unit": "ns",
            "value": 21249.620506626117
          },
          {
            "name": "astraweave-ecs::component_add/single_component/1000",
            "unit": "ns",
            "value": 192010.06495759095
          },
          {
            "name": "astraweave-ecs::component_add/single_component/10000",
            "unit": "ns",
            "value": 1840566.6008
          },
          {
            "name": "astraweave-ecs::component_iteration/position_write/100",
            "unit": "ns",
            "value": 2835.9448932172513
          },
          {
            "name": "astraweave-ecs::component_iteration/position_write/1000",
            "unit": "ns",
            "value": 27983.279521361706
          },
          {
            "name": "astraweave-ecs::component_iteration/position_write/10000",
            "unit": "ns",
            "value": 278571.5096391624
          },
          {
            "name": "astraweave-ecs::component_remove/multiple_components/100",
            "unit": "ns",
            "value": 91542.77079858792
          },
          {
            "name": "astraweave-ecs::component_remove/multiple_components/1000",
            "unit": "ns",
            "value": 855973.8346388079
          },
          {
            "name": "astraweave-ecs::component_remove/multiple_components/10000",
            "unit": "ns",
            "value": 8640809.94
          },
          {
            "name": "astraweave-ecs::component_remove/single_component/100",
            "unit": "ns",
            "value": 22013.647014830232
          },
          {
            "name": "astraweave-ecs::component_remove/single_component/1000",
            "unit": "ns",
            "value": 191906.59376581802
          },
          {
            "name": "astraweave-ecs::component_remove/single_component/10000",
            "unit": "ns",
            "value": 1884460.2742857144
          },
          {
            "name": "astraweave-ecs::entity_despawn/empty/100",
            "unit": "ns",
            "value": 1825.9568385647437
          },
          {
            "name": "astraweave-ecs::entity_despawn/empty/1000",
            "unit": "ns",
            "value": 12969.074278727343
          },
          {
            "name": "astraweave-ecs::entity_despawn/empty/10000",
            "unit": "ns",
            "value": 123463.53312059585
          },
          {
            "name": "astraweave-ecs::entity_despawn/with_components/100",
            "unit": "ns",
            "value": 12338.398489912948
          },
          {
            "name": "astraweave-ecs::entity_despawn/with_components/1000",
            "unit": "ns",
            "value": 121570.74357393931
          },
          {
            "name": "astraweave-ecs::entity_despawn/with_components/10000",
            "unit": "ns",
            "value": 1148026.40375
          },
          {
            "name": "astraweave-ecs::entity_insert/BTreeMap/100",
            "unit": "ns",
            "value": 3090.765212088121
          },
          {
            "name": "astraweave-ecs::entity_insert/BTreeMap/1000",
            "unit": "ns",
            "value": 57550.61065588941
          },
          {
            "name": "astraweave-ecs::entity_insert/BTreeMap/10000",
            "unit": "ns",
            "value": 712568.4237862878
          },
          {
            "name": "astraweave-ecs::entity_insert/SparseSet/100",
            "unit": "ns",
            "value": 779.8356416088058
          },
          {
            "name": "astraweave-ecs::entity_insert/SparseSet/1000",
            "unit": "ns",
            "value": 3399.06003477581
          },
          {
            "name": "astraweave-ecs::entity_insert/SparseSet/10000",
            "unit": "ns",
            "value": 38047.085781351576
          },
          {
            "name": "astraweave-ecs::entity_lookup/BTreeMap/100",
            "unit": "ns",
            "value": 1079.2993693629262
          },
          {
            "name": "astraweave-ecs::entity_lookup/BTreeMap/1000",
            "unit": "ns",
            "value": 14528.591300436112
          },
          {
            "name": "astraweave-ecs::entity_lookup/BTreeMap/10000",
            "unit": "ns",
            "value": 433956.5405453178
          },
          {
            "name": "astraweave-ecs::entity_lookup/SparseSet/100",
            "unit": "ns",
            "value": 77.40502479001891
          },
          {
            "name": "astraweave-ecs::entity_lookup/SparseSet/1000",
            "unit": "ns",
            "value": 662.6468783173649
          },
          {
            "name": "astraweave-ecs::entity_lookup/SparseSet/10000",
            "unit": "ns",
            "value": 6580.494671826132
          },
          {
            "name": "astraweave-ecs::entity_remove/BTreeMap/100",
            "unit": "ns",
            "value": 1946.3660077367447
          },
          {
            "name": "astraweave-ecs::entity_remove/BTreeMap/1000",
            "unit": "ns",
            "value": 23373.403217534174
          },
          {
            "name": "astraweave-ecs::entity_remove/BTreeMap/10000",
            "unit": "ns",
            "value": 261973.58992816365
          },
          {
            "name": "astraweave-ecs::entity_remove/SparseSet/100",
            "unit": "ns",
            "value": 269.4665411982428
          },
          {
            "name": "astraweave-ecs::entity_remove/SparseSet/1000",
            "unit": "ns",
            "value": 1978.1120936113934
          },
          {
            "name": "astraweave-ecs::entity_remove/SparseSet/10000",
            "unit": "ns",
            "value": 20514.490236560385
          },
          {
            "name": "astraweave-ecs::entity_spawn/empty/100",
            "unit": "ns",
            "value": 2547.564310767308
          },
          {
            "name": "astraweave-ecs::entity_spawn/empty/1000",
            "unit": "ns",
            "value": 15966.170797062208
          },
          {
            "name": "astraweave-ecs::entity_spawn/empty/10000",
            "unit": "ns",
            "value": 156234.10304338878
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position/100",
            "unit": "ns",
            "value": 22287.494785600837
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position/1000",
            "unit": "ns",
            "value": 197695.87518096936
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position/10000",
            "unit": "ns",
            "value": 1932603.0247664643
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position_velocity/100",
            "unit": "ns",
            "value": 57659.36461092027
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position_velocity/1000",
            "unit": "ns",
            "value": 556563.2250904642
          },
          {
            "name": "astraweave-ecs::entity_spawn/with_position_velocity/10000",
            "unit": "ns",
            "value": 5468013.256
          },
          {
            "name": "astraweave-ecs::sparseset_data/insert/100",
            "unit": "ns",
            "value": 1202.999639289269
          },
          {
            "name": "astraweave-ecs::sparseset_data/insert/1000",
            "unit": "ns",
            "value": 5746.806345847072
          },
          {
            "name": "astraweave-ecs::sparseset_data/insert/10000",
            "unit": "ns",
            "value": 50338.596860195335
          },
          {
            "name": "astraweave-ecs::sparseset_data/iterate/100",
            "unit": "ns",
            "value": 49.959167957765764
          },
          {
            "name": "astraweave-ecs::sparseset_data/iterate/1000",
            "unit": "ns",
            "value": 634.3032127146965
          },
          {
            "name": "astraweave-ecs::sparseset_data/iterate/10000",
            "unit": "ns",
            "value": 6532.042465377162
          },
          {
            "name": "astraweave-ecs::sparseset_data/mutate/100",
            "unit": "ns",
            "value": 96.72600580110773
          },
          {
            "name": "astraweave-ecs::sparseset_data/mutate/1000",
            "unit": "ns",
            "value": 961.8527646040836
          },
          {
            "name": "astraweave-ecs::sparseset_data/mutate/10000",
            "unit": "ns",
            "value": 9615.365335846533
          },
          {
            "name": "astraweave-ecs::storage_iteration/BlobVec_slice/100",
            "unit": "ns",
            "value": 58.98931442657971
          },
          {
            "name": "astraweave-ecs::storage_iteration/BlobVec_slice/1000",
            "unit": "ns",
            "value": 658.0218567779327
          },
          {
            "name": "astraweave-ecs::storage_iteration/BlobVec_slice/10000",
            "unit": "ns",
            "value": 6565.412370317571
          },
          {
            "name": "astraweave-ecs::storage_iteration/Vec_Box_downcast/100",
            "unit": "ns",
            "value": 222.3351385053098
          },
          {
            "name": "astraweave-ecs::storage_iteration/Vec_Box_downcast/1000",
            "unit": "ns",
            "value": 2607.7688877404494
          },
          {
            "name": "astraweave-ecs::storage_iteration/Vec_Box_downcast/10000",
            "unit": "ns",
            "value": 26487.492288364647
          },
          {
            "name": "astraweave-ecs::storage_mutation/BlobVec_slice_mut/100",
            "unit": "ns",
            "value": 44.87298922591796
          },
          {
            "name": "astraweave-ecs::storage_mutation/BlobVec_slice_mut/1000",
            "unit": "ns",
            "value": 448.9407771232935
          },
          {
            "name": "astraweave-ecs::storage_mutation/BlobVec_slice_mut/10000",
            "unit": "ns",
            "value": 5667.952392411976
          },
          {
            "name": "astraweave-ecs::storage_mutation/Vec_Box_downcast_mut/100",
            "unit": "ns",
            "value": 173.63010888108562
          },
          {
            "name": "astraweave-ecs::storage_mutation/Vec_Box_downcast_mut/1000",
            "unit": "ns",
            "value": 1642.3856807088177
          },
          {
            "name": "astraweave-ecs::storage_mutation/Vec_Box_downcast_mut/10000",
            "unit": "ns",
            "value": 15172.033824758451
          },
          {
            "name": "astraweave-ecs::storage_push/BlobVec/100",
            "unit": "ns",
            "value": 225.19601325807145
          },
          {
            "name": "astraweave-ecs::storage_push/BlobVec/1000",
            "unit": "ns",
            "value": 1252.952100611543
          },
          {
            "name": "astraweave-ecs::storage_push/BlobVec/10000",
            "unit": "ns",
            "value": 11201.154051265818
          },
          {
            "name": "astraweave-ecs::storage_push/Vec_Box/100",
            "unit": "ns",
            "value": 3875.906993274795
          },
          {
            "name": "astraweave-ecs::storage_push/Vec_Box/1000",
            "unit": "ns",
            "value": 41367.79319017654
          },
          {
            "name": "astraweave-ecs::storage_push/Vec_Box/10000",
            "unit": "ns",
            "value": 417406.6776320049
          },
          {
            "name": "astraweave-math::mat4_inverse_scalar",
            "unit": "ns",
            "value": 1.9604145812300242
          },
          {
            "name": "astraweave-math::mat4_inverse_simd",
            "unit": "ns",
            "value": 1.959990101264792
          },
          {
            "name": "astraweave-math::mat4_multiply_scalar",
            "unit": "ns",
            "value": 1.6338667943210188
          },
          {
            "name": "astraweave-math::mat4_multiply_simd",
            "unit": "ns",
            "value": 6.5815906697634565
          },
          {
            "name": "astraweave-math::mat4_transpose_scalar",
            "unit": "ns",
            "value": 1.9638655341747369
          },
          {
            "name": "astraweave-math::mat4_transpose_simd",
            "unit": "ns",
            "value": 1.9599527274113056
          },
          {
            "name": "astraweave-math::movement/naive/100",
            "unit": "ns",
            "value": 156.92883400569787
          },
          {
            "name": "astraweave-math::movement/naive/1000",
            "unit": "ns",
            "value": 1635.1317886854388
          },
          {
            "name": "astraweave-math::movement/naive/10000",
            "unit": "ns",
            "value": 16413.006293485818
          },
          {
            "name": "astraweave-math::movement/simd/100",
            "unit": "ns",
            "value": 49.59859049130548
          },
          {
            "name": "astraweave-math::movement/simd/1000",
            "unit": "ns",
            "value": 475.00085281168504
          },
          {
            "name": "astraweave-math::movement/simd/10000",
            "unit": "ns",
            "value": 5527.001173455501
          },
          {
            "name": "astraweave-math::physics_tick_scalar",
            "unit": "ns",
            "value": 1885.7001413100197
          },
          {
            "name": "astraweave-math::physics_tick_simd",
            "unit": "ns",
            "value": 2943.443459213988
          },
          {
            "name": "astraweave-math::quat_dot_scalar",
            "unit": "ns",
            "value": 0.6511326566001152
          },
          {
            "name": "astraweave-math::quat_dot_simd",
            "unit": "ns",
            "value": 0.6474521120234654
          },
          {
            "name": "astraweave-math::quat_multiply_scalar",
            "unit": "ns",
            "value": 0.35084538969232304
          },
          {
            "name": "astraweave-math::quat_multiply_simd",
            "unit": "ns",
            "value": 0.3448254517411227
          },
          {
            "name": "astraweave-math::quat_normalize_batch_scalar",
            "unit": "ns",
            "value": 31.33965107694079
          },
          {
            "name": "astraweave-math::quat_normalize_batch_simd",
            "unit": "ns",
            "value": 31.35833443939769
          },
          {
            "name": "astraweave-math::quat_normalize_scalar",
            "unit": "ns",
            "value": 0.6539490367872267
          },
          {
            "name": "astraweave-math::quat_normalize_simd",
            "unit": "ns",
            "value": 0.32846563791226485
          },
          {
            "name": "astraweave-math::quat_slerp_batch_scalar",
            "unit": "ns",
            "value": 273.8437580476978
          },
          {
            "name": "astraweave-math::quat_slerp_batch_simd",
            "unit": "ns",
            "value": 305.21691553399944
          },
          {
            "name": "astraweave-math::quat_slerp_scalar",
            "unit": "ns",
            "value": 0.32820267803784026
          },
          {
            "name": "astraweave-math::quat_slerp_simd",
            "unit": "ns",
            "value": 18.895988301801005
          },
          {
            "name": "astraweave-math::transform_point_scalar",
            "unit": "ns",
            "value": 0.9788269390580137
          },
          {
            "name": "astraweave-math::transform_point_simd",
            "unit": "ns",
            "value": 0.9783743178308484
          },
          {
            "name": "astraweave-math::transform_points_batch_scalar",
            "unit": "ns",
            "value": 23.231328256275717
          },
          {
            "name": "astraweave-math::transform_points_batch_simd",
            "unit": "ns",
            "value": 23.40297131247461
          },
          {
            "name": "astraweave-math::vec3_cross_scalar",
            "unit": "ns",
            "value": 11.562229382845663
          },
          {
            "name": "astraweave-math::vec3_cross_simd",
            "unit": "ns",
            "value": 12.431556576836321
          },
          {
            "name": "astraweave-math::vec3_dot_scalar",
            "unit": "ns",
            "value": 11.871447593225087
          },
          {
            "name": "astraweave-math::vec3_dot_simd",
            "unit": "ns",
            "value": 13.408820094180491
          },
          {
            "name": "astraweave-math::vec3_dot_throughput/scalar",
            "unit": "ns",
            "value": 10376.384868321062
          },
          {
            "name": "astraweave-math::vec3_dot_throughput/simd",
            "unit": "ns",
            "value": 11908.221845706383
          },
          {
            "name": "astraweave-math::vec3_length_scalar",
            "unit": "ns",
            "value": 14.73088297472766
          },
          {
            "name": "astraweave-math::vec3_length_simd",
            "unit": "ns",
            "value": 16.31682553549769
          },
          {
            "name": "astraweave-math::vec3_normalize_scalar",
            "unit": "ns",
            "value": 2.18355993715223
          },
          {
            "name": "astraweave-math::vec3_normalize_simd",
            "unit": "ns",
            "value": 20.814706699016018
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_compression/delta_compress_5000",
            "unit": "ns",
            "value": 4715.11773958881
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_compression/dictionary_compress_1000",
            "unit": "ns",
            "value": 16659.534650638736
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_compression/position_quantize_5000",
            "unit": "ns",
            "value": 0.6186974810331194
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_compression/rle_encode_10000",
            "unit": "ns",
            "value": 6960.87295511979
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_connection_management/broadcast_optimization_1000_clients",
            "unit": "ns",
            "value": 989.1647672245848
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_connection_management/channel_management_100",
            "unit": "ns",
            "value": 15092.334345985635
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_connection_management/client_tracking_500",
            "unit": "ns",
            "value": 47178.98455004669
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_connection_management/reconnection_handling_100",
            "unit": "ns",
            "value": 432.2532365966321
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_deserialization/binary_deserialize_1000",
            "unit": "ns",
            "value": 3808.6895574251275
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_deserialization/entity_parse_500",
            "unit": "ns",
            "value": 128183.04965721974
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_deserialization/message_type_detection_1000",
            "unit": "ns",
            "value": 61806.355345769625
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_flow_control/backpressure_handling",
            "unit": "ns",
            "value": 587.0045789767616
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_flow_control/congestion_window_1000",
            "unit": "ns",
            "value": 5595.326517616859
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_flow_control/priority_scheduling_500",
            "unit": "ns",
            "value": 4217.433696265732
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_flow_control/rate_limiting_10000",
            "unit": "ns",
            "value": 228221.0818722248
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_message_handling/duplicate_detection_1000",
            "unit": "ns",
            "value": 16499.459918341192
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_message_handling/message_batching_2000",
            "unit": "ns",
            "value": 1857.9694960889183
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_message_handling/priority_queue_1000",
            "unit": "ns",
            "value": 687.7167784296771
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_message_handling/queue_processing_5000",
            "unit": "ns",
            "value": 5236.4278655183325
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/binary_serialize_1000",
            "unit": "ns",
            "value": 4433.566655827727
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/delta_serialize_1000",
            "unit": "ns",
            "value": 2531.339565224388
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/plan_serialize_100",
            "unit": "ns",
            "value": 9915.45266701093
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/snapshot_serialize/100",
            "unit": "ns",
            "value": 15852.508685893074
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/snapshot_serialize/1000",
            "unit": "ns",
            "value": 190352.8191064293
          },
          {
            "name": "astraweave-ipc::ipc_adversarial_serialization/snapshot_serialize/500",
            "unit": "ns",
            "value": 92978.74329733562
          },
          {
            "name": "astraweave-memory::memory_access_tracking/10",
            "unit": "ns",
            "value": 1012.8560637034632
          },
          {
            "name": "astraweave-memory::memory_access_tracking/25",
            "unit": "ns",
            "value": 2461.1355813253253
          },
          {
            "name": "astraweave-memory::memory_access_tracking/50",
            "unit": "ns",
            "value": 4969.38960422341
          },
          {
            "name": "astraweave-memory::memory_creation",
            "unit": "ns",
            "value": 138.22323234337924
          },
          {
            "name": "astraweave-memory::memory_importance_update",
            "unit": "ns",
            "value": 135.41243434519157
          },
          {
            "name": "astraweave-memory::memory_retrieval_by_id",
            "unit": "ns",
            "value": 3796.2301650823165
          },
          {
            "name": "astraweave-memory::memory_storage/10",
            "unit": "ns",
            "value": 3000.999815192791
          },
          {
            "name": "astraweave-memory::memory_storage/25",
            "unit": "ns",
            "value": 10233.531371133298
          },
          {
            "name": "astraweave-memory::memory_storage/50",
            "unit": "ns",
            "value": 29798.059951528678
          },
          {
            "name": "astraweave-destruction::brush_remesh",
            "unit": "ns",
            "value": 3202572.430625
          },
          {
            "name": "astraweave-destruction::debris_step",
            "unit": "ns",
            "value": 220.82685740140005
          },
          {
            "name": "astraweave-destruction::fracture_cut/planar",
            "unit": "ns",
            "value": 29.763361652141395
          },
          {
            "name": "astraweave-destruction::fracture_cut/voronoi_16",
            "unit": "ns",
            "value": 455.7471771789644
          },
          {
            "name": "astraweave-destruction::graph_solve/1024",
            "unit": "ns",
            "value": 13516831.445
          },
          {
            "name": "astraweave-destruction::graph_solve/256",
            "unit": "ns",
            "value": 877055.591493289
          },
          {
            "name": "astraweave-destruction::graph_solve/64",
            "unit": "ns",
            "value": 57363.97025592079
          },
          {
            "name": "astraweave-fluids::fluid_system_step/step_submit_wait/10000",
            "unit": "ns",
            "value": 128286841.11666666
          },
          {
            "name": "astraweave-fluids::fluid_system_step/step_submit_wait/20000",
            "unit": "ns",
            "value": 222908203.53333333
          },
          {
            "name": "astraweave-fluids::fluid_system_step/step_submit_wait/50000",
            "unit": "ns",
            "value": 522728388.6
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_adaptive_iterations/decision_boundary_stress",
            "unit": "ns",
            "value": 403339.3486465332
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_adaptive_iterations/error_tracking_1000",
            "unit": "ns",
            "value": 7418.095973971174
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_adaptive_iterations/iteration_update_10000",
            "unit": "ns",
            "value": 205421.8787693691
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_density_pressure/density_computation_5000",
            "unit": "ns",
            "value": 3725524.407142857
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_density_pressure/pressure_computation_5000",
            "unit": "ns",
            "value": 6538.283053076343
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_density_pressure/pressure_force_2000",
            "unit": "ns",
            "value": 2292326.9295454547
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_density_pressure/viscosity_force_2000",
            "unit": "ns",
            "value": 2207166.770833334
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/cell_index_buffer_10000",
            "unit": "ns",
            "value": 0.5453900083171128
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/combined_buffer_10000",
            "unit": "ns",
            "value": 0.5473963622110976
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/position_buffer_10000",
            "unit": "ns",
            "value": 0.5452942610390711
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/sort_prep_10000",
            "unit": "ns",
            "value": 109720.31717803515
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/uniform_buffer_prep",
            "unit": "ns",
            "value": 4.522672810928832
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_gpu_data_prep/velocity_buffer_10000",
            "unit": "ns",
            "value": 0.5453251899008732
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_lod_system/distance_calculation_100000",
            "unit": "ns",
            "value": 218207.89160552283
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_lod_system/lod_transition_tracking",
            "unit": "ns",
            "value": 69575.02972722851
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_lod_system/lod_update_10000",
            "unit": "ns",
            "value": 16992.092284685226
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_lod_system/reduction_factor_lookup_10000",
            "unit": "ns",
            "value": 10944.66938886072
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/auto_tune_decision_10000",
            "unit": "ns",
            "value": 75306.41643167617
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/budget_check_10000",
            "unit": "ns",
            "value": 183038.3254449172
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/controller_creation",
            "unit": "ns",
            "value": 0.5462176032682925
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/frame_recording_1000",
            "unit": "ns",
            "value": 37337.67458425772
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/full_frame_cycle_1000",
            "unit": "ns",
            "value": 39562.75871424746
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/gpu_config_parsing_1000",
            "unit": "ns",
            "value": 397728.46129355876
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/headroom_calculation_10000",
            "unit": "ns",
            "value": 194065.63837288547
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_optimization_controller/iteration_recommendation_10000",
            "unit": "ns",
            "value": 0.8179889475327758
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/boundary_collision_10000",
            "unit": "ns",
            "value": 16400.9005307996
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/force_accumulation_10000",
            "unit": "ns",
            "value": 5473.153838775985
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/particle_creation/1000",
            "unit": "ns",
            "value": 2525.7610900419986
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/particle_creation/10000",
            "unit": "ns",
            "value": 25246.46222895916
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/particle_creation/5000",
            "unit": "ns",
            "value": 12535.110999894918
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/position_update_10000",
            "unit": "ns",
            "value": 6562.416386939353
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_particle_operations/velocity_update_10000",
            "unit": "ns",
            "value": 12602.680472220718
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/aos_access_pattern_10000",
            "unit": "ns",
            "value": 8157.010094555358
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/density_4x_unroll_10000",
            "unit": "ns",
            "value": 3517.159427163506
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/density_8x_unroll_10000",
            "unit": "ns",
            "value": 24573.24686686018
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/distance_squared_only_100000",
            "unit": "ns",
            "value": 573650.9908614424
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/distance_with_sqrt_100000",
            "unit": "ns",
            "value": 139933.84960496463
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/kernel_with_early_out_10000",
            "unit": "ns",
            "value": 11150.681940223487
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/kernel_without_early_out_10000",
            "unit": "ns",
            "value": 16470.795438988654
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/morton_code_generation_100000",
            "unit": "ns",
            "value": 391318.767534723
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/soa_access_pattern_10000",
            "unit": "ns",
            "value": 24441.462890560557
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/weighted_centroid_fma_1000",
            "unit": "ns",
            "value": 7180.397118810577
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simd_optimizations/weighted_centroid_naive_1000",
            "unit": "ns",
            "value": 818.523651824079
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simulation_step/adaptive_timestep_1000",
            "unit": "ns",
            "value": 1666.861706017216
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simulation_step/multi_step_500_x10",
            "unit": "ns",
            "value": 163137.7685357735
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_simulation_step/single_step_1000",
            "unit": "ns",
            "value": 1030724.1799233493
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/cell_density_analysis",
            "unit": "ns",
            "value": 12129.41344420591
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/grid_construction/1000",
            "unit": "ns",
            "value": 78631.4035482628
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/grid_construction/10000",
            "unit": "ns",
            "value": 881718.2962752526
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/grid_construction/5000",
            "unit": "ns",
            "value": 430375.6573606497
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/grid_rebuild_5000",
            "unit": "ns",
            "value": 304552.07037428254
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_spatial_hashing/neighbor_query_10000",
            "unit": "ns",
            "value": 6008395.310000001
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_sph_kernels/combined_kernels_10000",
            "unit": "ns",
            "value": 20616.104574663896
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_sph_kernels/poly6_kernel_100000",
            "unit": "ns",
            "value": 98648.2960849463
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_sph_kernels/spiky_gradient_100000",
            "unit": "ns",
            "value": 103888.71287904066
          },
          {
            "name": "astraweave-fluids::fluids_adversarial_sph_kernels/viscosity_laplacian_100000",
            "unit": "ns",
            "value": 92334.6130310855
          },
          {
            "name": "astraweave-scripting::script_execution/ecs_script_system_1000_entities",
            "unit": "ns",
            "value": 947012.4287001777
          },
          {
            "name": "astraweave-scripting::script_execution/rhai_raw_execution",
            "unit": "ns",
            "value": 329.5347057262123
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_callback_events/batch_events_100",
            "unit": "ns",
            "value": 4.9771711300166315
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_callback_events/empty_event_queue",
            "unit": "ns",
            "value": 0.5451388424972601
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_callback_events/event_filtering_by_entity",
            "unit": "ns",
            "value": 126.42430471919884
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_callback_events/high_freq_collisions_500",
            "unit": "ns",
            "value": 9482.246504164807
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_callback_events/single_event_dispatch",
            "unit": "ns",
            "value": 5.9950452110265235
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_command_processing/batch_commands_100",
            "unit": "ns",
            "value": 1792.3930870022448
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_command_processing/command_validation_100",
            "unit": "ns",
            "value": 109.90549831209859
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_command_processing/empty_command_list",
            "unit": "ns",
            "value": 0.5459743018468702
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_command_processing/mixed_commands_50",
            "unit": "ns",
            "value": 444.63595138275923
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_command_processing/single_spawn_command",
            "unit": "ns",
            "value": 17.08066348076846
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/deeply_nested_100",
            "unit": "ns",
            "value": 40.938805574799616
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/empty_script_compile",
            "unit": "ns",
            "value": 10.387244975873596
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/large_script_compile/100",
            "unit": "ns",
            "value": 93.32823405799768
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/large_script_compile/1000",
            "unit": "ns",
            "value": 606.0841687494237
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/large_script_compile/500",
            "unit": "ns",
            "value": 316.359235074163
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/many_functions_50",
            "unit": "ns",
            "value": 89.85959266333607
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_compilation_stress/syntax_error_handling",
            "unit": "ns",
            "value": 15.022883286545689
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_execution_stress/loop_heavy_1000_iterations",
            "unit": "ns",
            "value": 57634.08658812771
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_execution_stress/many_commands_100",
            "unit": "ns",
            "value": 2922.1580077646695
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_execution_stress/noop_execution",
            "unit": "ns",
            "value": 12.235836788245958
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_execution_stress/simple_execution",
            "unit": "ns",
            "value": 163.0694669567596
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_execution_stress/state_persistence_stress",
            "unit": "ns",
            "value": 65.62417151193695
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_hot_reload/cache_hit_no_reload",
            "unit": "ns",
            "value": 3.5426875684088093
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_hot_reload/cache_invalidation_100",
            "unit": "ns",
            "value": 159.07257566250723
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_hot_reload/modification_detection",
            "unit": "ns",
            "value": 0.5451463245125889
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_hot_reload/recompilation_cycle",
            "unit": "ns",
            "value": 53.05970832698458
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_hot_reload/state_preservation",
            "unit": "ns",
            "value": 58.975227182206005
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/array_size_limit",
            "unit": "ns",
            "value": 21.96199964691534
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/combined_limits_check",
            "unit": "ns",
            "value": 1.3615461529016262
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/expr_depth_check",
            "unit": "ns",
            "value": 21.688649265849136
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/operation_limit_check",
            "unit": "ns",
            "value": 0.6227396394800013
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/operation_limit_exceeded",
            "unit": "ns",
            "value": 0.5458922560610151
          },
          {
            "name": "astraweave-scripting::scripting_adversarial_security_limits/string_size_limit",
            "unit": "ns",
            "value": 225.0331394523012
          },
          {
            "name": "astraweave-author::author_adversarial_conversion/map_to_json_100",
            "unit": "ns",
            "value": 13980.028567796015
          },
          {
            "name": "astraweave-author::author_adversarial_conversion/nested_structure_50",
            "unit": "ns",
            "value": 34950.917875366045
          },
          {
            "name": "astraweave-author::author_adversarial_conversion/toml_parsing_200_lines",
            "unit": "ns",
            "value": 23772.09235367734
          },
          {
            "name": "astraweave-author::author_adversarial_conversion/value_serialization_500",
            "unit": "ns",
            "value": 28531.184430769434
          },
          {
            "name": "astraweave-author::author_adversarial_integration/event_dispatching_500",
            "unit": "ns",
            "value": 7926.24655896831
          },
          {
            "name": "astraweave-author::author_adversarial_integration/full_map_pipeline",
            "unit": "ns",
            "value": 1281.9872418068464
          },
          {
            "name": "astraweave-author::author_adversarial_integration/script_entity_spawning_100",
            "unit": "ns",
            "value": 3000.6031915817484
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/biome_distribution_1024",
            "unit": "ns",
            "value": 5871.321847401884
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/director_budget_calculation",
            "unit": "ns",
            "value": 0.5459072377765206
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/map_meta_creation/128x128",
            "unit": "ns",
            "value": 399.2479555692025
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/map_meta_creation/256x256",
            "unit": "ns",
            "value": 398.63894043756244
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/map_meta_creation/64x64",
            "unit": "ns",
            "value": 398.03717878781055
          },
          {
            "name": "astraweave-author::author_adversarial_map_generation/spawn_zone_generation_100",
            "unit": "ns",
            "value": 10119.44040912506
          },
          {
            "name": "astraweave-author::author_adversarial_sandboxing/forbidden_api_detection",
            "unit": "ns",
            "value": 3613.312841533419
          },
          {
            "name": "astraweave-author::author_adversarial_sandboxing/memory_tracking_allocations",
            "unit": "ns",
            "value": 13348.478235125931
          },
          {
            "name": "astraweave-author::author_adversarial_sandboxing/operation_counting_10000",
            "unit": "ns",
            "value": 0.5451784104613282
          },
          {
            "name": "astraweave-author::author_adversarial_sandboxing/stack_depth_limit_recursive",
            "unit": "ns",
            "value": 0.5451071454056213
          },
          {
            "name": "astraweave-author::author_adversarial_sandboxing/time_limit_check_1000",
            "unit": "ns",
            "value": 485.58078847181235
          },
          {
            "name": "astraweave-author::author_adversarial_script_execution/context_switching_100",
            "unit": "ns",
            "value": 0.5466983688007881
          },
          {
            "name": "astraweave-author::author_adversarial_script_execution/function_calls_500",
            "unit": "ns",
            "value": 9343.233956075912
          },
          {
            "name": "astraweave-author::author_adversarial_script_execution/loop_execution_10000_iterations",
            "unit": "ns",
            "value": 287017.45284967654
          },
          {
            "name": "astraweave-author::author_adversarial_script_execution/variable_operations_1000",
            "unit": "ns",
            "value": 69185.97098514247
          },
          {
            "name": "astraweave-author::author_adversarial_script_parsing/malicious_detection_50",
            "unit": "ns",
            "value": 5164.501449817356
          },
          {
            "name": "astraweave-author::author_adversarial_script_parsing/parse_expressions_1000",
            "unit": "ns",
            "value": 28738.565299317335
          },
          {
            "name": "astraweave-author::author_adversarial_script_parsing/token_counting_large_script",
            "unit": "ns",
            "value": 10322.849592837274
          },
          {
            "name": "astraweave-author::author_adversarial_script_parsing/validate_scripts_100",
            "unit": "ns",
            "value": 25478.289179968553
          },
          {
            "name": "astraweave-core::entity_spawning",
            "unit": "ns",
            "value": 25977.877813931616
          },
          {
            "name": "astraweave-core::full_game_loop/large_1000e_100a",
            "unit": "ns",
            "value": 1634648.2871775655
          },
          {
            "name": "astraweave-core::full_game_loop/medium_500e_50a",
            "unit": "ns",
            "value": 426758.7751413764
          },
          {
            "name": "astraweave-core::full_game_loop/small_100e_10a",
            "unit": "ns",
            "value": 21700.38543895474
          },
          {
            "name": "astraweave-core::full_game_loop/stress_5000e_500a",
            "unit": "ns",
            "value": 40878186.52
          },
          {
            "name": "astraweave-core::full_game_loop_multi_frame/1000e_60f",
            "unit": "ns",
            "value": 4203082.695
          },
          {
            "name": "astraweave-core::full_game_loop_multi_frame/100e_60f",
            "unit": "ns",
            "value": 390234.01718536817
          },
          {
            "name": "astraweave-core::full_game_loop_multi_frame/500e_60f",
            "unit": "ns",
            "value": 2033757.9796000004
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/100",
            "unit": "ns",
            "value": 5816.0054921664705
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/1000",
            "unit": "ns",
            "value": 64029.568259768115
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/2000",
            "unit": "ns",
            "value": 132647.92929757506
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/250",
            "unit": "ns",
            "value": 15041.286979466917
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/50",
            "unit": "ns",
            "value": 2926.9832544767055
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/500",
            "unit": "ns",
            "value": 30635.81032705458
          },
          {
            "name": "astraweave-core::full_game_loop_scaling/5000",
            "unit": "ns",
            "value": 390679.84064458526
          },
          {
            "name": "astraweave-core::full_game_loop_single_frame/1000_entities",
            "unit": "ns",
            "value": 63644.613322893136
          },
          {
            "name": "astraweave-core::full_game_loop_single_frame/100_entities",
            "unit": "ns",
            "value": 5717.8858416630155
          },
          {
            "name": "astraweave-core::full_game_loop_single_frame/5000_entities",
            "unit": "ns",
            "value": 408583.63829304365
          },
          {
            "name": "astraweave-core::full_game_loop_single_frame/500_entities",
            "unit": "ns",
            "value": 30719.065752793304
          },
          {
            "name": "astraweave-core::perception_stage/100_agents",
            "unit": "ns",
            "value": 1538230.1734604302
          },
          {
            "name": "astraweave-core::perception_stage/10_agents",
            "unit": "ns",
            "value": 15796.759344466635
          },
          {
            "name": "astraweave-core::perception_stage/50_agents",
            "unit": "ns",
            "value": 388550.182364976
          },
          {
            "name": "astraweave-core::physics_stage/100_agents",
            "unit": "ns",
            "value": 121.4805251909853
          },
          {
            "name": "astraweave-core::planning_stage/100_agents",
            "unit": "ns",
            "value": 10212.804973980328
          },
          {
            "name": "astraweave-core::rendering_prep/1000_entities",
            "unit": "ns",
            "value": 18531.494029440346
          },
          {
            "name": "astraweave-core::rendering_prep/100_entities",
            "unit": "ns",
            "value": 1636.4257499008193
          },
          {
            "name": "astraweave-core::rendering_prep/500_entities",
            "unit": "ns",
            "value": 8924.48368842258
          },
          {
            "name": "astraweave-core::world_creation",
            "unit": "ns",
            "value": 34.702482636576484
          },
          {
            "name": "astraweave-core::world_tick",
            "unit": "ns",
            "value": 41.34945819345684
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/compressed/10",
            "unit": "ns",
            "value": 341.03928509120766
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/compressed/100",
            "unit": "ns",
            "value": 862.4413082098167
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/compressed/1000",
            "unit": "ns",
            "value": 3991.5023149985605
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/compressed/50",
            "unit": "ns",
            "value": 618.43061207998
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/compressed/500",
            "unit": "ns",
            "value": 2298.1785255653845
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/uncompressed/10",
            "unit": "ns",
            "value": 79.3700607429115
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/uncompressed/100",
            "unit": "ns",
            "value": 311.0438698034825
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/uncompressed/1000",
            "unit": "ns",
            "value": 1794.5541413348392
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/uncompressed/50",
            "unit": "ns",
            "value": 215.75491661702338
          },
          {
            "name": "astraweave-net-ecs::bandwidth_delta_size/uncompressed/500",
            "unit": "ns",
            "value": 984.1994300218373
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/compressed/10",
            "unit": "ns",
            "value": 799.2888793513126
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/compressed/100",
            "unit": "ns",
            "value": 3942.1788818364685
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/compressed/1000",
            "unit": "ns",
            "value": 32617.0389951417
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/compressed/50",
            "unit": "ns",
            "value": 2389.1365212112773
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/compressed/500",
            "unit": "ns",
            "value": 18652.38063398406
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/uncompressed/10",
            "unit": "ns",
            "value": 304.64853837779
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/uncompressed/100",
            "unit": "ns",
            "value": 1748.1175653060768
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/uncompressed/1000",
            "unit": "ns",
            "value": 16522.254249745645
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/uncompressed/50",
            "unit": "ns",
            "value": 1020.5225133524046
          },
          {
            "name": "astraweave-net-ecs::bandwidth_snapshot_size/uncompressed/500",
            "unit": "ns",
            "value": 7695.521924445188
          },
          {
            "name": "astraweave-net-ecs::client_server_input_processing/1",
            "unit": "ns",
            "value": 132794.3283842277
          },
          {
            "name": "astraweave-net-ecs::client_server_input_processing/10",
            "unit": "ns",
            "value": 423956.8861133012
          },
          {
            "name": "astraweave-net-ecs::client_server_input_processing/100",
            "unit": "ns",
            "value": 2177451.8083953545
          },
          {
            "name": "astraweave-net-ecs::client_server_input_processing/50",
            "unit": "ns",
            "value": 870801.2289859573
          },
          {
            "name": "astraweave-net-ecs::client_server_reconciliation/1",
            "unit": "ns",
            "value": 712.918508229886
          },
          {
            "name": "astraweave-net-ecs::client_server_reconciliation/10",
            "unit": "ns",
            "value": 6722.505023634538
          },
          {
            "name": "astraweave-net-ecs::client_server_reconciliation/100",
            "unit": "ns",
            "value": 67194.35689173876
          },
          {
            "name": "astraweave-net-ecs::client_server_reconciliation/50",
            "unit": "ns",
            "value": 33250.20006788259
          },
          {
            "name": "astraweave-net-ecs::client_server_snapshot_generation/1",
            "unit": "ns",
            "value": 396.8447064815458
          },
          {
            "name": "astraweave-net-ecs::client_server_snapshot_generation/10",
            "unit": "ns",
            "value": 849.0628961167785
          },
          {
            "name": "astraweave-net-ecs::client_server_snapshot_generation/100",
            "unit": "ns",
            "value": 7534.307777313703
          },
          {
            "name": "astraweave-net-ecs::client_server_snapshot_generation/50",
            "unit": "ns",
            "value": 3324.260945926557
          },
          {
            "name": "astraweave-net-ecs::compression/compress_entity_batch",
            "unit": "ns",
            "value": 8296.025179085587
          },
          {
            "name": "astraweave-net-ecs::compression/compression_ratio_analysis",
            "unit": "ns",
            "value": 8301.821612754173
          },
          {
            "name": "astraweave-net-ecs::compression/decompress_entity_batch",
            "unit": "ns",
            "value": 12860.736003501188
          },
          {
            "name": "astraweave-net-ecs::compression/delta_then_compress_100",
            "unit": "ns",
            "value": 7898.825253605784
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_compress/10",
            "unit": "ns",
            "value": 460.70732128314506
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_compress/100",
            "unit": "ns",
            "value": 2547.4415559111603
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_compress/50",
            "unit": "ns",
            "value": 1486.967763052696
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_compress/500",
            "unit": "ns",
            "value": 8441.150854354883
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_decompress/10",
            "unit": "ns",
            "value": 127.95859510993907
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_decompress/100",
            "unit": "ns",
            "value": 468.7204281367156
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_decompress/50",
            "unit": "ns",
            "value": 279.08077107038866
          },
          {
            "name": "astraweave-net-ecs::compression_snapshot/lz4_decompress/500",
            "unit": "ns",
            "value": 1343.9930630557637
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/apply_delta/10",
            "unit": "ns",
            "value": 56.08912052538794
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/apply_delta/100",
            "unit": "ns",
            "value": 364.6030084843687
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/apply_delta/50",
            "unit": "ns",
            "value": 211.70905020573372
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/apply_delta/500",
            "unit": "ns",
            "value": 1689.1599338366702
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/compute_delta/10",
            "unit": "ns",
            "value": 206.07951685988425
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/compute_delta/100",
            "unit": "ns",
            "value": 2044.6465043329636
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/compute_delta/50",
            "unit": "ns",
            "value": 1029.3620936309128
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/compute_delta/500",
            "unit": "ns",
            "value": 10495.455469872808
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/serialize_delta/10",
            "unit": "ns",
            "value": 83.35833707647578
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/serialize_delta/100",
            "unit": "ns",
            "value": 335.08686253232037
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/serialize_delta/50",
            "unit": "ns",
            "value": 203.6478278723115
          },
          {
            "name": "astraweave-net-ecs::delta_encoding/serialize_delta/500",
            "unit": "ns",
            "value": 979.3816096229999
          },
          {
            "name": "astraweave-net-ecs::deserialization/deserialize_transforms_1000",
            "unit": "ns",
            "value": 1233.3264588777142
          },
          {
            "name": "astraweave-net-ecs::deserialization/parse_packet_header",
            "unit": "ns",
            "value": 1.3327219762080293
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/delta_compression_1000",
            "unit": "ns",
            "value": 11674.482360205317
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/dirty_serialize/10",
            "unit": "ns",
            "value": 1341.7317636863418
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/dirty_serialize/100",
            "unit": "ns",
            "value": 15415.558721912807
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/dirty_serialize/1000",
            "unit": "ns",
            "value": 164735.00543848154
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/dirty_serialize/10000",
            "unit": "ns",
            "value": 1374124.563173295
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/full_serialize/10",
            "unit": "ns",
            "value": 582.5598603881155
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/full_serialize/100",
            "unit": "ns",
            "value": 7440.762780704015
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/full_serialize/1000",
            "unit": "ns",
            "value": 70050.33147902851
          },
          {
            "name": "astraweave-net-ecs::entity_serialization/full_serialize/10000",
            "unit": "ns",
            "value": 712077.1211959267
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_delta_cycle/10",
            "unit": "ns",
            "value": 766.3506949515738
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_delta_cycle/100",
            "unit": "ns",
            "value": 4037.291323677966
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_delta_cycle/50",
            "unit": "ns",
            "value": 2228.232653380399
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_delta_cycle/500",
            "unit": "ns",
            "value": 17650.43903750416
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_sync_cycle/10",
            "unit": "ns",
            "value": 1299.6607238044382
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_sync_cycle/100",
            "unit": "ns",
            "value": 8072.521463518954
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_sync_cycle/50",
            "unit": "ns",
            "value": 4464.874216959262
          },
          {
            "name": "astraweave-net-ecs::full_pipeline_sync_cycle/500",
            "unit": "ns",
            "value": 37282.844494177334
          },
          {
            "name": "astraweave-net-ecs::interest_management/get_relevant/100",
            "unit": "ns",
            "value": 236.88320228595822
          },
          {
            "name": "astraweave-net-ecs::interest_management/get_relevant/1000",
            "unit": "ns",
            "value": 2244.358615403615
          },
          {
            "name": "astraweave-net-ecs::interest_management/get_relevant/10000",
            "unit": "ns",
            "value": 28882.819403731846
          },
          {
            "name": "astraweave-net-ecs::interest_management/multi_client_interest_1000_entities_16_clients",
            "unit": "ns",
            "value": 150735.20585519253
          },
          {
            "name": "astraweave-net-ecs::packet_batching/create_packet/10",
            "unit": "ns",
            "value": 1192.5653759919235
          },
          {
            "name": "astraweave-net-ecs::packet_batching/create_packet/100",
            "unit": "ns",
            "value": 10726.287154712047
          },
          {
            "name": "astraweave-net-ecs::packet_batching/create_packet/50",
            "unit": "ns",
            "value": 5397.9631740996665
          },
          {
            "name": "astraweave-net-ecs::packet_batching/create_packet/500",
            "unit": "ns",
            "value": 41419.38942436478
          },
          {
            "name": "astraweave-net-ecs::packet_batching/multi_packet_frame_100_entities_4_clients",
            "unit": "ns",
            "value": 33383.012210780835
          },
          {
            "name": "astraweave-net-ecs::serialization_entity_state/deserialize_postcard",
            "unit": "ns",
            "value": 30.43606899134716
          },
          {
            "name": "astraweave-net-ecs::serialization_entity_state/serialize_postcard",
            "unit": "ns",
            "value": 56.44701608329075
          },
          {
            "name": "astraweave-net-ecs::serialization_snapshot/postcard/10",
            "unit": "ns",
            "value": 310.5760834175583
          },
          {
            "name": "astraweave-net-ecs::serialization_snapshot/postcard/100",
            "unit": "ns",
            "value": 1820.4937541948136
          },
          {
            "name": "astraweave-net-ecs::serialization_snapshot/postcard/50",
            "unit": "ns",
            "value": 1024.771180369733
          },
          {
            "name": "astraweave-net-ecs::serialization_snapshot/postcard/500",
            "unit": "ns",
            "value": 8347.207338047861
          },
          {
            "name": "astraweave-net-ecs::snapshot_interpolation/interpolate_100_entities/10",
            "unit": "ns",
            "value": 12887.559736382904
          },
          {
            "name": "astraweave-net-ecs::snapshot_interpolation/interpolate_100_entities/128",
            "unit": "ns",
            "value": 262226.4184845503
          },
          {
            "name": "astraweave-net-ecs::snapshot_interpolation/interpolate_100_entities/32",
            "unit": "ns",
            "value": 46584.77453593075
          },
          {
            "name": "astraweave-net-ecs::snapshot_interpolation/interpolate_100_entities/64",
            "unit": "ns",
            "value": 108646.34468026641
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/full_world_sync/100",
            "unit": "ns",
            "value": 21520.12834587116
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/full_world_sync/1000",
            "unit": "ns",
            "value": 201779.46153317828
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/full_world_sync/500",
            "unit": "ns",
            "value": 102474.78506595374
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/partial_update_20pct/100",
            "unit": "ns",
            "value": 2159.305184296889
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/partial_update_20pct/1000",
            "unit": "ns",
            "value": 17232.889463567888
          },
          {
            "name": "astraweave-net-ecs::state_synchronization/partial_update_20pct/500",
            "unit": "ns",
            "value": 10681.857361505257
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_batch_processing/memory_budget_tracking",
            "unit": "ns",
            "value": 6831.39344002847
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_batch_processing/parallel_compression_20",
            "unit": "ns",
            "value": 22218.340610653082
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_batch_processing/parallel_mesh_opt_20",
            "unit": "ns",
            "value": 604378.9895636608
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_batch_processing/pipeline_scheduling_100",
            "unit": "ns",
            "value": 111442.90933029984
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_format_conversion/index_delta_encoding_30000",
            "unit": "ns",
            "value": 22458.77319516463
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_format_conversion/normal_octahedral_10000",
            "unit": "ns",
            "value": 43245.28254057594
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_format_conversion/tangent_compression_5000",
            "unit": "ns",
            "value": 18733.659167092912
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_format_conversion/vertex_format_conversion_10000",
            "unit": "ns",
            "value": 0.6232927715549124
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_lod_generation/edge_collapse_selection",
            "unit": "ns",
            "value": 321633.113227585
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_lod_generation/lod_level_generation_4",
            "unit": "ns",
            "value": 20845.789146884053
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_lod_generation/quadric_error_5000_verts",
            "unit": "ns",
            "value": 153895.48948083157
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_lod_generation/screen_space_error_100",
            "unit": "ns",
            "value": 977.546924810562
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/index_buffer_opt_10000",
            "unit": "ns",
            "value": 168413.2505694254
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/overdraw_analysis_1000_tris",
            "unit": "ns",
            "value": 2387.4673637536794
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/vertex_cache_opt/1000",
            "unit": "ns",
            "value": 14075.862810421359
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/vertex_cache_opt/10000",
            "unit": "ns",
            "value": 145219.1868769847
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/vertex_cache_opt/5000",
            "unit": "ns",
            "value": 70874.44706085506
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_mesh_optimization/vertex_deduplication_5000",
            "unit": "ns",
            "value": 273056.8320480192
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/astc_compression/Astc4x4",
            "unit": "ns",
            "value": 2959.3716333219754
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/astc_compression/Astc6x6",
            "unit": "ns",
            "value": 1381.9846453759362
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/astc_compression/Astc8x8",
            "unit": "ns",
            "value": 802.1936671142817
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/bc7_compression/1024x1024",
            "unit": "ns",
            "value": 0.6223300928092361
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/bc7_compression/256x256",
            "unit": "ns",
            "value": 0.6226628301874139
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/bc7_compression/512x512",
            "unit": "ns",
            "value": 0.6224061478570087
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/bc7_compression/64x64",
            "unit": "ns",
            "value": 0.6222991568207302
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/mipmap_generation_1024",
            "unit": "ns",
            "value": 99783.60837510077
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/npot_texture_300x200",
            "unit": "ns",
            "value": 66506.79575249665
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_texture_compression/quality_preset_selection",
            "unit": "ns",
            "value": 0.6231230002437722
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_validation/animation_validation_20",
            "unit": "ns",
            "value": 3114.7839077090925
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_validation/material_ref_validation",
            "unit": "ns",
            "value": 6904.288522092032
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_validation/mesh_validation_50",
            "unit": "ns",
            "value": 49499.00736047821
          },
          {
            "name": "astraweave-asset-pipeline::pipeline_adversarial_validation/texture_validation_100",
            "unit": "ns",
            "value": 1697.6106356874325
          },
          {
            "name": "astraweave-prompts::batch_render/10",
            "unit": "ns",
            "value": 118358.35878313804
          },
          {
            "name": "astraweave-prompts::batch_render/100",
            "unit": "ns",
            "value": 1114577.1277645107
          },
          {
            "name": "astraweave-prompts::batch_render/50",
            "unit": "ns",
            "value": 562281.4005978331
          },
          {
            "name": "astraweave-prompts::context_add_variables/10",
            "unit": "ns",
            "value": 1278.393827528954
          },
          {
            "name": "astraweave-prompts::context_add_variables/20",
            "unit": "ns",
            "value": 2819.270277894266
          },
          {
            "name": "astraweave-prompts::context_add_variables/5",
            "unit": "ns",
            "value": 575.0781390893994
          },
          {
            "name": "astraweave-prompts::context_clone",
            "unit": "ns",
            "value": 616.7216562710802
          },
          {
            "name": "astraweave-prompts::context_creation_complex",
            "unit": "ns",
            "value": 1759.9383341870125
          },
          {
            "name": "astraweave-prompts::context_creation_simple",
            "unit": "ns",
            "value": 135.4456162094732
          },
          {
            "name": "astraweave-prompts::context_to_string_map",
            "unit": "ns",
            "value": 1003.1038084296752
          },
          {
            "name": "astraweave-prompts::engine_creation",
            "unit": "ns",
            "value": 2638.464067448568
          },
          {
            "name": "astraweave-prompts::engine_register_template/1",
            "unit": "ns",
            "value": 66046.28481008849
          },
          {
            "name": "astraweave-prompts::engine_register_template/10",
            "unit": "ns",
            "value": 649539.6726874558
          },
          {
            "name": "astraweave-prompts::engine_register_template/50",
            "unit": "ns",
            "value": 3274344.825625
          },
          {
            "name": "astraweave-prompts::engine_render",
            "unit": "ns",
            "value": 1118.793547260871
          },
          {
            "name": "astraweave-prompts::template_clone",
            "unit": "ns",
            "value": 235.5742833867742
          },
          {
            "name": "astraweave-prompts::template_creation_complex",
            "unit": "ns",
            "value": 56899.67262577092
          },
          {
            "name": "astraweave-prompts::template_creation_dialogue",
            "unit": "ns",
            "value": 53670.43184857333
          },
          {
            "name": "astraweave-prompts::template_creation_simple",
            "unit": "ns",
            "value": 52398.11554893967
          },
          {
            "name": "astraweave-prompts::template_render_complex",
            "unit": "ns",
            "value": 44447.473535989135
          },
          {
            "name": "astraweave-prompts::template_render_dialogue",
            "unit": "ns",
            "value": 24437.14639798539
          },
          {
            "name": "astraweave-prompts::template_render_map",
            "unit": "ns",
            "value": 11839.179878581992
          },
          {
            "name": "astraweave-prompts::template_render_simple",
            "unit": "ns",
            "value": 10864.733157120718
          },
          {
            "name": "astraweave-audio::audio_engine_new",
            "unit": "ns",
            "value": 667.7713919178433
          },
          {
            "name": "astraweave-audio::audio_tick/0_sources",
            "unit": "ns",
            "value": 10.605044275916134
          },
          {
            "name": "astraweave-audio::audio_tick/100_sources",
            "unit": "ns",
            "value": 10.381248182274005
          },
          {
            "name": "astraweave-audio::audio_tick/10_sources",
            "unit": "ns",
            "value": 9.995960829649064
          },
          {
            "name": "astraweave-audio::audio_tick/50_sources",
            "unit": "ns",
            "value": 10.323031451773028
          },
          {
            "name": "astraweave-audio::beep_generation/3d_beep",
            "unit": "ns",
            "value": 36.49464939565675
          },
          {
            "name": "astraweave-audio::beep_generation/sfx_beep",
            "unit": "ns",
            "value": 4.663670594027649
          },
          {
            "name": "astraweave-audio::beep_generation/voice_beep",
            "unit": "ns",
            "value": 7.824883807992822
          },
          {
            "name": "astraweave-audio::spatial_audio/listener_movement_10_emitters",
            "unit": "ns",
            "value": 71.23936560588547
          },
          {
            "name": "astraweave-audio::spatial_audio/listener_movement_single_emitter",
            "unit": "ns",
            "value": 66.93504907395351
          },
          {
            "name": "astraweave-audio::spatial_audio/pan_mode_switching",
            "unit": "ns",
            "value": 0.3112424943358627
          },
          {
            "name": "astraweave-audio::volume_control/master_volume_set",
            "unit": "ns",
            "value": 8.446504530740611
          },
          {
            "name": "astraweave-audio::volume_control/master_volume_with_active_sounds",
            "unit": "ns",
            "value": 18.72127893221264
          },
          {
            "name": "astraweave-persistence-ecs::blob_size/10",
            "unit": "ns",
            "value": 6554.658429926142
          },
          {
            "name": "astraweave-persistence-ecs::blob_size/100",
            "unit": "ns",
            "value": 53926.541401642135
          },
          {
            "name": "astraweave-persistence-ecs::blob_size/1000",
            "unit": "ns",
            "value": 540937.3663863613
          },
          {
            "name": "astraweave-persistence-ecs::blob_size/2000",
            "unit": "ns",
            "value": 1067852.358522325
          },
          {
            "name": "astraweave-persistence-ecs::blob_size/500",
            "unit": "ns",
            "value": 266619.55474641
          },
          {
            "name": "astraweave-persistence-ecs::calculate_hash/10",
            "unit": "ns",
            "value": 2290.803352557237
          },
          {
            "name": "astraweave-persistence-ecs::calculate_hash/100",
            "unit": "ns",
            "value": 22656.361825287888
          },
          {
            "name": "astraweave-persistence-ecs::calculate_hash/1000",
            "unit": "ns",
            "value": 341850.23360270076
          },
          {
            "name": "astraweave-persistence-ecs::calculate_hash/2000",
            "unit": "ns",
            "value": 953941.5603786738
          },
          {
            "name": "astraweave-persistence-ecs::calculate_hash/500",
            "unit": "ns",
            "value": 137822.94899326487
          },
          {
            "name": "astraweave-persistence-ecs::checksum_verification/crc32_compute/102400bytes",
            "unit": "ns",
            "value": 796393.1718129773
          },
          {
            "name": "astraweave-persistence-ecs::checksum_verification/crc32_compute/10240bytes",
            "unit": "ns",
            "value": 79598.40415338545
          },
          {
            "name": "astraweave-persistence-ecs::checksum_verification/crc32_compute/1024bytes",
            "unit": "ns",
            "value": 7950.824970164963
          },
          {
            "name": "astraweave-persistence-ecs::checksum_verification/crc32_compute/1048576bytes",
            "unit": "ns",
            "value": 8203542.768571425
          },
          {
            "name": "astraweave-persistence-ecs::checksum_verification/verify_snapshot_integrity_1000",
            "unit": "ns",
            "value": 488000.3056769171
          },
          {
            "name": "astraweave-persistence-ecs::component_deserialization/deserialize_healths_1000",
            "unit": "ns",
            "value": 1065.3585155489034
          },
          {
            "name": "astraweave-persistence-ecs::component_deserialization/deserialize_inventories_100",
            "unit": "ns",
            "value": 44554.515158878305
          },
          {
            "name": "astraweave-persistence-ecs::component_deserialization/deserialize_transforms_1000",
            "unit": "ns",
            "value": 1456.5247556500153
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/compress/102400bytes",
            "unit": "ns",
            "value": 148474.74791969953
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/compress/10240bytes",
            "unit": "ns",
            "value": 14779.66090590954
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/compress/1024bytes",
            "unit": "ns",
            "value": 1492.1548959583465
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/compress/1048576bytes",
            "unit": "ns",
            "value": 1529222.3340044108
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/decompress/102400bytes",
            "unit": "ns",
            "value": 138866.15420403218
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/decompress/10240bytes",
            "unit": "ns",
            "value": 14471.074263470373
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/decompress/1024bytes",
            "unit": "ns",
            "value": 1694.0275762004028
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/decompress/1048576bytes",
            "unit": "ns",
            "value": 1432949.1678045916
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/ratio_analysis/102400bytes",
            "unit": "ns",
            "value": 151580.44667832728
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/ratio_analysis/10240bytes",
            "unit": "ns",
            "value": 15283.250052966992
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/ratio_analysis/1024bytes",
            "unit": "ns",
            "value": 1685.893372476043
          },
          {
            "name": "astraweave-persistence-ecs::compression_efficiency/ratio_analysis/1048576bytes",
            "unit": "ns",
            "value": 1553042.838704311
          },
          {
            "name": "astraweave-persistence-ecs::deserialize_world/10",
            "unit": "ns",
            "value": 18811.266213711075
          },
          {
            "name": "astraweave-persistence-ecs::deserialize_world/100",
            "unit": "ns",
            "value": 120144.30662922819
          },
          {
            "name": "astraweave-persistence-ecs::deserialize_world/1000",
            "unit": "ns",
            "value": 1025483.7319484918
          },
          {
            "name": "astraweave-persistence-ecs::deserialize_world/2000",
            "unit": "ns",
            "value": 2015665.3120000004
          },
          {
            "name": "astraweave-persistence-ecs::deserialize_world/500",
            "unit": "ns",
            "value": 533161.1939631528
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/deserialize/10",
            "unit": "ns",
            "value": 1131.3693463055163
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/deserialize/100",
            "unit": "ns",
            "value": 16272.656727753643
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/deserialize/1000",
            "unit": "ns",
            "value": 157148.0430185605
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/deserialize/50",
            "unit": "ns",
            "value": 8337.159375099336
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/deserialize/500",
            "unit": "ns",
            "value": 79553.82076023963
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/serialize/10",
            "unit": "ns",
            "value": 691.2306327993912
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/serialize/100",
            "unit": "ns",
            "value": 4852.569852108228
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/serialize/1000",
            "unit": "ns",
            "value": 46761.370963959314
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/serialize/50",
            "unit": "ns",
            "value": 2523.448845644957
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_entity_batch/serialize/500",
            "unit": "ns",
            "value": 23282.564231367127
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_single_component/deserialize_health",
            "unit": "ns",
            "value": 9.646750607892342
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_single_component/deserialize_position",
            "unit": "ns",
            "value": 9.496119657248677
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_single_component/serialize_health",
            "unit": "ns",
            "value": 35.455152909427454
          },
          {
            "name": "astraweave-persistence-ecs::ecs_serialization_single_component/serialize_position",
            "unit": "ns",
            "value": 35.70637291320743
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_entities/100",
            "unit": "ns",
            "value": 12767.794474193592
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_entities/1000",
            "unit": "ns",
            "value": 166073.38126709437
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_entities/10000",
            "unit": "ns",
            "value": 1685048.9803429274
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_with_compression/100",
            "unit": "ns",
            "value": 44752.48233639569
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_with_compression/1000",
            "unit": "ns",
            "value": 450928.3023353954
          },
          {
            "name": "astraweave-persistence-ecs::entity_serialization/serialize_with_compression/10000",
            "unit": "ns",
            "value": 4543356.897500003
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/compute_delta/10pct_change",
            "unit": "ns",
            "value": 6600.249814761592
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/compute_delta/1pct_change",
            "unit": "ns",
            "value": 804.4338847243473
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/compute_delta/25pct_change",
            "unit": "ns",
            "value": 14977.219141693733
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/compute_delta/50pct_change",
            "unit": "ns",
            "value": 29163.39154704927
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/compute_delta/5pct_change",
            "unit": "ns",
            "value": 2633.9444298441654
          },
          {
            "name": "astraweave-persistence-ecs::incremental_delta/delta_vs_full_comparison",
            "unit": "ns",
            "value": 203874.46587682865
          },
          {
            "name": "astraweave-persistence-ecs::persistence_manager_operations/list_saves",
            "unit": "ns",
            "value": 8125.327263036603
          },
          {
            "name": "astraweave-persistence-ecs::persistence_manager_operations/load_game",
            "unit": "ns",
            "value": 24116.17736437942
          },
          {
            "name": "astraweave-persistence-ecs::persistence_manager_operations/save_game",
            "unit": "ns",
            "value": 544220.1680655592
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/deserialize/10",
            "unit": "ns",
            "value": 1236.5478566338095
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/deserialize/100",
            "unit": "ns",
            "value": 14897.37613295157
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/deserialize/50",
            "unit": "ns",
            "value": 7627.7299118453675
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/deserialize/500",
            "unit": "ns",
            "value": 73592.09004332812
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/serialize/10",
            "unit": "ns",
            "value": 776.7297487423681
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/serialize/100",
            "unit": "ns",
            "value": 5943.403194234852
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/serialize/50",
            "unit": "ns",
            "value": 3175.965910809163
          },
          {
            "name": "astraweave-persistence-ecs::replay_operations/serialize/500",
            "unit": "ns",
            "value": 30839.521815644915
          },
          {
            "name": "astraweave-persistence-ecs::replay_tick_advance/advance_tick",
            "unit": "ns",
            "value": 38.09321308895612
          },
          {
            "name": "astraweave-persistence-ecs::roundtrip_world/10",
            "unit": "ns",
            "value": 25802.89845268226
          },
          {
            "name": "astraweave-persistence-ecs::roundtrip_world/100",
            "unit": "ns",
            "value": 177834.02014798514
          },
          {
            "name": "astraweave-persistence-ecs::roundtrip_world/1000",
            "unit": "ns",
            "value": 1567624.7614730252
          },
          {
            "name": "astraweave-persistence-ecs::roundtrip_world/2000",
            "unit": "ns",
            "value": 3094621.6111764694
          },
          {
            "name": "astraweave-persistence-ecs::roundtrip_world/500",
            "unit": "ns",
            "value": 809830.5678855906
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/load/10",
            "unit": "ns",
            "value": 13471.450749348453
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/load/100",
            "unit": "ns",
            "value": 22819.856935037693
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/load/50",
            "unit": "ns",
            "value": 17793.033077440796
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/load/500",
            "unit": "ns",
            "value": 57806.87017904206
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/save/10",
            "unit": "ns",
            "value": 524673.9502166314
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/save/100",
            "unit": "ns",
            "value": 538154.075773656
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/save/50",
            "unit": "ns",
            "value": 530880.8433473725
          },
          {
            "name": "astraweave-persistence-ecs::save_load_cycle_full_pipeline/save/500",
            "unit": "ns",
            "value": 631516.2909152496
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_load/100",
            "unit": "ns",
            "value": 20424.911659914032
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_load/1000",
            "unit": "ns",
            "value": 100242.21069168924
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_load/500",
            "unit": "ns",
            "value": 55818.03111583539
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_load/5000",
            "unit": "ns",
            "value": 454821.8163959971
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_save/100",
            "unit": "ns",
            "value": 525643.4372947037
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_save/1000",
            "unit": "ns",
            "value": 706842.5096760835
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_save/500",
            "unit": "ns",
            "value": 624625.4413710902
          },
          {
            "name": "astraweave-persistence-ecs::scaling_entity_count/full_save/5000",
            "unit": "ns",
            "value": 1389569.7207286106
          },
          {
            "name": "astraweave-persistence-ecs::serialize_world/10",
            "unit": "ns",
            "value": 6519.554323664393
          },
          {
            "name": "astraweave-persistence-ecs::serialize_world/100",
            "unit": "ns",
            "value": 53444.44721485922
          },
          {
            "name": "astraweave-persistence-ecs::serialize_world/1000",
            "unit": "ns",
            "value": 534013.4329603241
          },
          {
            "name": "astraweave-persistence-ecs::serialize_world/2000",
            "unit": "ns",
            "value": 1055999.80823828
          },
          {
            "name": "astraweave-persistence-ecs::serialize_world/500",
            "unit": "ns",
            "value": 263893.7088461573
          },
          {
            "name": "astraweave-persistence-ecs::version_migration/migrate_transforms_add_field_1000",
            "unit": "ns",
            "value": 80114.33196697308
          },
          {
            "name": "astraweave-persistence-ecs::version_migration/version_check_1000",
            "unit": "ns",
            "value": 705.2118401263433
          },
          {
            "name": "astraweave-persistence-ecs::world_hash_calculation/10",
            "unit": "ns",
            "value": 267.1317935261975
          },
          {
            "name": "astraweave-persistence-ecs::world_hash_calculation/100",
            "unit": "ns",
            "value": 2538.228695105215
          },
          {
            "name": "astraweave-persistence-ecs::world_hash_calculation/1000",
            "unit": "ns",
            "value": 25294.171261875446
          },
          {
            "name": "astraweave-persistence-ecs::world_hash_calculation/50",
            "unit": "ns",
            "value": 1275.9150515657593
          },
          {
            "name": "astraweave-persistence-ecs::world_hash_calculation/500",
            "unit": "ns",
            "value": 12645.803805879095
          },
          {
            "name": "astraweave-persistence-ecs::world_snapshot/checksum_1000_entities",
            "unit": "ns",
            "value": 499118.18791881227
          },
          {
            "name": "astraweave-persistence-ecs::world_snapshot/create_snapshot/100",
            "unit": "ns",
            "value": 76172.99838485186
          },
          {
            "name": "astraweave-persistence-ecs::world_snapshot/create_snapshot/1000",
            "unit": "ns",
            "value": 755543.0176624218
          },
          {
            "name": "astraweave-persistence-ecs::world_snapshot/create_snapshot/500",
            "unit": "ns",
            "value": 382849.123067656
          },
          {
            "name": "astraweave-persistence-ecs::world_snapshot/create_snapshot/5000",
            "unit": "ns",
            "value": 3767589.9771428574
          },
          {
            "name": "astraweave-weaving::adjudication/adjudicate_10_intents",
            "unit": "ns",
            "value": 726.7731735852117
          },
          {
            "name": "astraweave-weaving::adjudication/adjudicate_5_intents",
            "unit": "ns",
            "value": 159.19911013316406
          },
          {
            "name": "astraweave-weaving::adjudication/adjudicate_with_cooldowns",
            "unit": "ns",
            "value": 159.21214005677655
          },
          {
            "name": "astraweave-weaving::adjudication/begin_tick",
            "unit": "ns",
            "value": 8.107609778518135
          },
          {
            "name": "astraweave-weaving::adjudication/budget_check",
            "unit": "ns",
            "value": 0.622402700488122
          },
          {
            "name": "astraweave-weaving::adjudication/cooldown_check",
            "unit": "ns",
            "value": 0.6222910205354816
          },
          {
            "name": "astraweave-weaving::configuration/config_creation",
            "unit": "ns",
            "value": 310.34837026229803
          },
          {
            "name": "astraweave-weaving::configuration/config_from_toml",
            "unit": "ns",
            "value": 2792.3592539984425
          },
          {
            "name": "astraweave-weaving::configuration/config_to_toml",
            "unit": "ns",
            "value": 1282.9675449136332
          },
          {
            "name": "astraweave-weaving::enemy_spawner/determine_archetype/1",
            "unit": "ns",
            "value": 5.782671037518771
          },
          {
            "name": "astraweave-weaving::enemy_spawner/determine_archetype/10",
            "unit": "ns",
            "value": 5.805658941656503
          },
          {
            "name": "astraweave-weaving::enemy_spawner/determine_archetype/15",
            "unit": "ns",
            "value": 5.794244399654824
          },
          {
            "name": "astraweave-weaving::enemy_spawner/determine_archetype/20",
            "unit": "ns",
            "value": 5.799950581378616
          },
          {
            "name": "astraweave-weaving::enemy_spawner/determine_archetype/5",
            "unit": "ns",
            "value": 5.799010070772835
          },
          {
            "name": "astraweave-weaving::full_pipeline/full_weave_cycle",
            "unit": "ns",
            "value": 800.6438137528914
          },
          {
            "name": "astraweave-weaving::full_pipeline/pipeline_scaling/10",
            "unit": "ns",
            "value": 312.97193806216603
          },
          {
            "name": "astraweave-weaving::full_pipeline/pipeline_scaling/100",
            "unit": "ns",
            "value": 548.1107203080398
          },
          {
            "name": "astraweave-weaving::full_pipeline/pipeline_scaling/50",
            "unit": "ns",
            "value": 549.7195135883766
          },
          {
            "name": "astraweave-weaving::integrated_systems/10",
            "unit": "ns",
            "value": 51.77370967763949
          },
          {
            "name": "astraweave-weaving::integrated_systems/100",
            "unit": "ns",
            "value": 571.0446845366937
          },
          {
            "name": "astraweave-weaving::integrated_systems/50",
            "unit": "ns",
            "value": 238.86581355972467
          },
          {
            "name": "astraweave-weaving::intent_proposal/aid_event_proposal",
            "unit": "ns",
            "value": 111.37475553304822
          },
          {
            "name": "astraweave-weaving::intent_proposal/intent_builder",
            "unit": "ns",
            "value": 147.20988539475607
          },
          {
            "name": "astraweave-weaving::intent_proposal/multiple_proposers",
            "unit": "ns",
            "value": 415.1268152377016
          },
          {
            "name": "astraweave-weaving::intent_proposal/supply_drop_proposal",
            "unit": "ns",
            "value": 291.65398892790625
          },
          {
            "name": "astraweave-weaving::pattern_detection/low_health_cluster",
            "unit": "ns",
            "value": 38.11895220727929
          },
          {
            "name": "astraweave-weaving::pattern_detection/multiple_detectors",
            "unit": "ns",
            "value": 156.9023786956715
          },
          {
            "name": "astraweave-weaving::pattern_detection/pattern_strength_categorization",
            "unit": "ns",
            "value": 1.2454449810580013
          },
          {
            "name": "astraweave-weaving::pattern_detection/resource_scarcity",
            "unit": "ns",
            "value": 91.87258785733398
          },
          {
            "name": "astraweave-weaving::player_abilities/1",
            "unit": "ns",
            "value": 3.7528422971185664
          },
          {
            "name": "astraweave-weaving::player_abilities/10",
            "unit": "ns",
            "value": 22.484700927498302
          },
          {
            "name": "astraweave-weaving::player_abilities/100",
            "unit": "ns",
            "value": 227.53960083461695
          },
          {
            "name": "astraweave-weaving::player_abilities/1000",
            "unit": "ns",
            "value": 2301.7189968054263
          },
          {
            "name": "astraweave-weaving::player_ability_activation/dash_activation",
            "unit": "ns",
            "value": 154.1897728804584
          },
          {
            "name": "astraweave-weaving::player_ability_activation/shield_activation",
            "unit": "ns",
            "value": 152.325548516301
          },
          {
            "name": "astraweave-weaving::quest_objectives/1",
            "unit": "ns",
            "value": 159.6569782078687
          },
          {
            "name": "astraweave-weaving::quest_objectives/10",
            "unit": "ns",
            "value": 1494.398938039064
          },
          {
            "name": "astraweave-weaving::quest_objectives/100",
            "unit": "ns",
            "value": 14982.967281299707
          },
          {
            "name": "astraweave-weaving::quest_objectives/50",
            "unit": "ns",
            "value": 7438.245392450422
          },
          {
            "name": "astraweave-observability::observability_adversarial_crash_reporting/backtrace_generation_100",
            "unit": "ns",
            "value": 1076147.402432814
          },
          {
            "name": "astraweave-observability::observability_adversarial_crash_reporting/crash_report_creation_500",
            "unit": "ns",
            "value": 3207574.0425
          },
          {
            "name": "astraweave-observability::observability_adversarial_crash_reporting/report_serialization_200",
            "unit": "ns",
            "value": 50775.50865246376
          },
          {
            "name": "astraweave-observability::observability_adversarial_crash_reporting/symbolication_1000",
            "unit": "ns",
            "value": 58156.17510038259
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/log_entry_creation/1000",
            "unit": "ns",
            "value": 426263.65460490773
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/log_entry_creation/10000",
            "unit": "ns",
            "value": 4386009.856666666
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/log_entry_creation/5000",
            "unit": "ns",
            "value": 2199463.879565217
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/log_formatting_5000",
            "unit": "ns",
            "value": 1458423.5390114712
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/log_level_filtering_10000",
            "unit": "ns",
            "value": 20202.144907294085
          },
          {
            "name": "astraweave-observability::observability_adversarial_logging/target_matching_10000",
            "unit": "ns",
            "value": 95391.90806761476
          },
          {
            "name": "astraweave-observability::observability_adversarial_metrics_collection/counter_increments_100000",
            "unit": "ns",
            "value": 6082953.079999999
          },
          {
            "name": "astraweave-observability::observability_adversarial_metrics_collection/gauge_updates_50000",
            "unit": "ns",
            "value": 1142128.4932062896
          },
          {
            "name": "astraweave-observability::observability_adversarial_metrics_collection/histogram_observations_10000",
            "unit": "ns",
            "value": 71979.00542680238
          },
          {
            "name": "astraweave-observability::observability_adversarial_metrics_collection/metric_aggregation_5000",
            "unit": "ns",
            "value": 411374.6463439337
          },
          {
            "name": "astraweave-observability::observability_adversarial_metrics_collection/metric_tagging_10000",
            "unit": "ns",
            "value": 4752590.84
          },
          {
            "name": "astraweave-observability::observability_adversarial_performance_monitoring/cpu_profile_aggregation_5000",
            "unit": "ns",
            "value": 197238.18233769323
          },
          {
            "name": "astraweave-observability::observability_adversarial_performance_monitoring/fps_calculation_600",
            "unit": "ns",
            "value": 8893.350051355508
          },
          {
            "name": "astraweave-observability::observability_adversarial_performance_monitoring/gpu_timing_aggregation_1000",
            "unit": "ns",
            "value": 25545.611542012106
          },
          {
            "name": "astraweave-observability::observability_adversarial_performance_monitoring/memory_tracking_1000",
            "unit": "ns",
            "value": 19964.603558543782
          },
          {
            "name": "astraweave-observability::observability_adversarial_span_operations/nested_span_tree_depth_20",
            "unit": "ns",
            "value": 512473.2871535665
          },
          {
            "name": "astraweave-observability::observability_adversarial_span_operations/span_attributes_20000",
            "unit": "ns",
            "value": 1765215.9252930614
          },
          {
            "name": "astraweave-observability::observability_adversarial_span_operations/span_creation_10000",
            "unit": "ns",
            "value": 916355.4517063457
          },
          {
            "name": "astraweave-observability::observability_adversarial_span_operations/span_lifecycle_5000",
            "unit": "ns",
            "value": 1642089.9021877619
          },
          {
            "name": "astraweave-observability::observability_adversarial_trace_context/baggage_propagation_2000",
            "unit": "ns",
            "value": 647402.7880093198
          },
          {
            "name": "astraweave-observability::observability_adversarial_trace_context/context_creation_10000",
            "unit": "ns",
            "value": 0.6226666167540936
          },
          {
            "name": "astraweave-observability::observability_adversarial_trace_context/header_encoding_5000",
            "unit": "ns",
            "value": 1189815.0105544582
          },
          {
            "name": "astraweave-observability::observability_adversarial_trace_context/header_parsing_5000",
            "unit": "ns",
            "value": 621199.9603230404
          },
          {
            "name": "astraweave-input::action_insertion",
            "unit": "ns",
            "value": 696.2337764094233
          },
          {
            "name": "astraweave-input::binding_creation",
            "unit": "ns",
            "value": 6.062869118678361
          },
          {
            "name": "astraweave-input::binding_deserialization",
            "unit": "ns",
            "value": 132.208386823882
          },
          {
            "name": "astraweave-input::binding_lookup",
            "unit": "ns",
            "value": 13.914946340545844
          },
          {
            "name": "astraweave-input::binding_serialization",
            "unit": "ns",
            "value": 68.23301172211217
          },
          {
            "name": "astraweave-input::binding_set_clone",
            "unit": "ns",
            "value": 53.47508864102631
          },
          {
            "name": "astraweave-input::binding_set_creation",
            "unit": "ns",
            "value": 679.6549535723278
          },
          {
            "name": "astraweave-input::clear_frame",
            "unit": "ns",
            "value": 0.3115472742026647
          },
          {
            "name": "astraweave-input::context_switching",
            "unit": "ns",
            "value": 1.5557289348160241
          },
          {
            "name": "astraweave-input::input_manager_creation",
            "unit": "ns",
            "value": 71.4178911131075
          },
          {
            "name": "astraweave-input::is_down_query",
            "unit": "ns",
            "value": 0.6225626800481936
          },
          {
            "name": "astraweave-input::just_pressed_query",
            "unit": "ns",
            "value": 0.6222768083206539
          },
          {
            "name": "astraweave-input::multiple_queries",
            "unit": "ns",
            "value": 1.2469863872402513
          },
          {
            "name": "astraweave-input::sensitivity_access",
            "unit": "ns",
            "value": 0.6341284006105425
          },
          {
            "name": "astraweave-rag::memory_batch_clone/10",
            "unit": "ns",
            "value": 476.69230325925804
          },
          {
            "name": "astraweave-rag::memory_batch_clone/100",
            "unit": "ns",
            "value": 6101.671044125509
          },
          {
            "name": "astraweave-rag::memory_batch_clone/50",
            "unit": "ns",
            "value": 3240.120862136921
          },
          {
            "name": "astraweave-rag::memory_batch_creation/10",
            "unit": "ns",
            "value": 2150.174628975181
          },
          {
            "name": "astraweave-rag::memory_batch_creation/100",
            "unit": "ns",
            "value": 23477.05200858002
          },
          {
            "name": "astraweave-rag::memory_batch_creation/50",
            "unit": "ns",
            "value": 11697.342841458601
          },
          {
            "name": "astraweave-rag::memory_batch_creation/500",
            "unit": "ns",
            "value": 113402.62826848919
          },
          {
            "name": "astraweave-rag::memory_clone",
            "unit": "ns",
            "value": 44.009015774735374
          },
          {
            "name": "astraweave-rag::memory_creation",
            "unit": "ns",
            "value": 92.92948934586155
          },
          {
            "name": "astraweave-rag::memory_deserialize_json",
            "unit": "ns",
            "value": 392.75014855094656
          },
          {
            "name": "astraweave-rag::memory_serialize_json",
            "unit": "ns",
            "value": 241.76189693577427
          },
          {
            "name": "astraweave-rag::query_creation_complex",
            "unit": "ns",
            "value": 153.01119419146215
          },
          {
            "name": "astraweave-rag::query_creation_simple",
            "unit": "ns",
            "value": 22.562182594932747
          },
          {
            "name": "astraweave-rag::rag_adversarial_context_injection/empty_memories_injection",
            "unit": "ns",
            "value": 201.01200417742936
          },
          {
            "name": "astraweave-rag::rag_adversarial_context_injection/multiple_memories_10",
            "unit": "ns",
            "value": 1335.1794246907382
          },
          {
            "name": "astraweave-rag::rag_adversarial_context_injection/single_memory_injection",
            "unit": "ns",
            "value": 231.13939891667
          },
          {
            "name": "astraweave-rag::rag_adversarial_context_injection/token_limit_enforcement",
            "unit": "ns",
            "value": 360.15403762825184
          },
          {
            "name": "astraweave-rag::rag_adversarial_diversity_sampling/category_diversity_100",
            "unit": "ns",
            "value": 2411.330359268542
          },
          {
            "name": "astraweave-rag::rag_adversarial_diversity_sampling/mmr_sampling_50",
            "unit": "ns",
            "value": 87270.55560712652
          },
          {
            "name": "astraweave-rag::rag_adversarial_diversity_sampling/temporal_diversity_100",
            "unit": "ns",
            "value": 2535.674159937348
          },
          {
            "name": "astraweave-rag::rag_adversarial_forgetting_mechanisms/access_count_forgetting",
            "unit": "ns",
            "value": 328.0635796089235
          },
          {
            "name": "astraweave-rag::rag_adversarial_forgetting_mechanisms/combined_forgetting_policy",
            "unit": "ns",
            "value": 501.948569238095
          },
          {
            "name": "astraweave-rag::rag_adversarial_forgetting_mechanisms/importance_forgetting_100",
            "unit": "ns",
            "value": 280.66311789232265
          },
          {
            "name": "astraweave-rag::rag_adversarial_forgetting_mechanisms/time_decay_100",
            "unit": "ns",
            "value": 4320.068978413203
          },
          {
            "name": "astraweave-rag::rag_adversarial_memory_consolidation/consolidate_similar_20",
            "unit": "ns",
            "value": 12698.669977769017
          },
          {
            "name": "astraweave-rag::rag_adversarial_memory_consolidation/consolidation_priority_100",
            "unit": "ns",
            "value": 4016.6012092075307
          },
          {
            "name": "astraweave-rag::rag_adversarial_memory_consolidation/no_consolidation_unique_50",
            "unit": "ns",
            "value": 442256.84030373587
          },
          {
            "name": "astraweave-rag::rag_adversarial_query_processing/empty_query_handling",
            "unit": "ns",
            "value": 1.5553933528567412
          },
          {
            "name": "astraweave-rag::rag_adversarial_query_processing/long_query_5k_chars",
            "unit": "ns",
            "value": 40.585604350078434
          },
          {
            "name": "astraweave-rag::rag_adversarial_query_processing/multi_query_batch_10",
            "unit": "ns",
            "value": 287.81302988363944
          },
          {
            "name": "astraweave-rag::rag_adversarial_query_processing/query_expansion",
            "unit": "ns",
            "value": 208.99526228116477
          },
          {
            "name": "astraweave-rag::rag_adversarial_query_processing/query_normalization",
            "unit": "ns",
            "value": 321.01304739140164
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/empty_store_retrieval",
            "unit": "ns",
            "value": 7.469355510801111
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/high_threshold_100",
            "unit": "ns",
            "value": 44453.419152886054
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/store_retrieval/100",
            "unit": "ns",
            "value": 51141.549567146736
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/store_retrieval/1000",
            "unit": "ns",
            "value": 524781.3760616505
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/store_retrieval/500",
            "unit": "ns",
            "value": 266110.58315591014
          },
          {
            "name": "astraweave-rag::rag_adversarial_retrieval_stress/zero_threshold_100",
            "unit": "ns",
            "value": 54031.04239011978
          },
          {
            "name": "astraweave-rag::rag_config_creation",
            "unit": "ns",
            "value": 35.02990691884182
          },
          {
            "name": "astraweave-rag::rag_config_custom",
            "unit": "ns",
            "value": 34.06670629161034
          },
          {
            "name": "astraweave-rag::result_ranking/100",
            "unit": "ns",
            "value": 34588.275321128975
          },
          {
            "name": "astraweave-rag::result_ranking/200",
            "unit": "ns",
            "value": 68650.29706994048
          },
          {
            "name": "astraweave-rag::result_ranking/50",
            "unit": "ns",
            "value": 18015.659714797628
          },
          {
            "name": "astraweave-rag::retrieval_category_filtering",
            "unit": "ns",
            "value": 11212.525823143013
          },
          {
            "name": "astraweave-rag::retrieval_engine_creation",
            "unit": "ns",
            "value": 2.4878035452536746
          },
          {
            "name": "astraweave-rag::retrieval_search_scaling/100",
            "unit": "ns",
            "value": 8183.429234602722
          },
          {
            "name": "astraweave-rag::retrieval_search_scaling/1000",
            "unit": "ns",
            "value": 93667.73809672247
          },
          {
            "name": "astraweave-rag::retrieval_search_scaling/50",
            "unit": "ns",
            "value": 3810.5572704722254
          },
          {
            "name": "astraweave-rag::retrieval_search_scaling/500",
            "unit": "ns",
            "value": 46778.36303212352
          },
          {
            "name": "astraweave-rag::retrieval_simple_search",
            "unit": "ns",
            "value": 5975.327219262726
          },
          {
            "name": "astraweave-rag::similarity_calculation",
            "unit": "ns",
            "value": 376.05541521009843
          },
          {
            "name": "astraweave-llm-eval::batch_processing/concurrent_batches_10x100",
            "unit": "ns",
            "value": 648223.2728864916
          },
          {
            "name": "astraweave-llm-eval::batch_processing/process_batch/10",
            "unit": "ns",
            "value": 18670.731123098307
          },
          {
            "name": "astraweave-llm-eval::batch_processing/process_batch/100",
            "unit": "ns",
            "value": 195861.90857889148
          },
          {
            "name": "astraweave-llm-eval::batch_processing/process_batch/50",
            "unit": "ns",
            "value": 98696.02866628054
          },
          {
            "name": "astraweave-llm-eval::batch_processing/process_batch/500",
            "unit": "ns",
            "value": 934223.0347064602
          },
          {
            "name": "astraweave-llm-eval::config_operations/config_cloning_1000",
            "unit": "ns",
            "value": 248606.49352996706
          },
          {
            "name": "astraweave-llm-eval::config_operations/config_creation_100",
            "unit": "ns",
            "value": 15880.818546188646
          },
          {
            "name": "astraweave-llm-eval::evaluation_edge_cases/empty_responses",
            "unit": "ns",
            "value": 5794.905226617157
          },
          {
            "name": "astraweave-llm-eval::evaluation_edge_cases/long_prompts_10k_chars",
            "unit": "ns",
            "value": 202644.0638354878
          },
          {
            "name": "astraweave-llm-eval::evaluation_edge_cases/score_variance_computation",
            "unit": "ns",
            "value": 4374.564467486065
          },
          {
            "name": "astraweave-llm-eval::evaluation_edge_cases/unicode_heavy_prompts",
            "unit": "ns",
            "value": 26180.148287724343
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/aggregate_scores/100",
            "unit": "ns",
            "value": 559.5089113003777
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/aggregate_scores/1000",
            "unit": "ns",
            "value": 4409.793513597662
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/aggregate_scores/10000",
            "unit": "ns",
            "value": 42557.96255725204
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/aggregate_scores/100000",
            "unit": "ns",
            "value": 414964.8316625847
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/compute_percentiles/100",
            "unit": "ns",
            "value": 744.4366999958419
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/compute_percentiles/1000",
            "unit": "ns",
            "value": 5934.067182672855
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/compute_percentiles/10000",
            "unit": "ns",
            "value": 56879.10243423133
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/compute_percentiles/100000",
            "unit": "ns",
            "value": 555634.5643173017
          },
          {
            "name": "astraweave-llm-eval::metric_aggregation/multi_metric_1000_samples",
            "unit": "ns",
            "value": 171856.06694446318
          },
          {
            "name": "astraweave-llm-eval::prompt_generation/complex_prompt_templates",
            "unit": "ns",
            "value": 928777.3968478686
          },
          {
            "name": "astraweave-llm-eval::prompt_generation/create_prompts/100",
            "unit": "ns",
            "value": 33751.5627430275
          },
          {
            "name": "astraweave-llm-eval::prompt_generation/create_prompts/1000",
            "unit": "ns",
            "value": 351629.51621120685
          },
          {
            "name": "astraweave-llm-eval::prompt_generation/create_prompts/10000",
            "unit": "ns",
            "value": 3505550.959333335
          },
          {
            "name": "astraweave-llm-eval::response_evaluation/full_pipeline_100",
            "unit": "ns",
            "value": 74430.5860290517
          },
          {
            "name": "astraweave-llm-eval::response_evaluation/score_responses/10",
            "unit": "ns",
            "value": 317.1819785813649
          },
          {
            "name": "astraweave-llm-eval::response_evaluation/score_responses/100",
            "unit": "ns",
            "value": 4802.014949168057
          },
          {
            "name": "astraweave-llm-eval::response_evaluation/score_responses/1000",
            "unit": "ns",
            "value": 52296.30850428612
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/cosine_long_text",
            "unit": "ns",
            "value": 632810.2994085831
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/cosine_similarity_100",
            "unit": "ns",
            "value": 238309.06024224978
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/jaccard_long_text",
            "unit": "ns",
            "value": 504982.16497507313
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/jaccard_similarity_100",
            "unit": "ns",
            "value": 255261.6046274194
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/levenshtein_distance_100",
            "unit": "ns",
            "value": 1633512.0825249278
          },
          {
            "name": "astraweave-llm-eval::similarity_calculations/normalized_edit_distance_100",
            "unit": "ns",
            "value": 1617817.2921211047
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_audit_logging/audit_entry_creation_50000",
            "unit": "ns",
            "value": 11559772.038000003
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_audit_logging/audit_filter_by_actor_20000",
            "unit": "ns",
            "value": 102853.33850479213
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_audit_logging/failed_operation_tracking_10000",
            "unit": "ns",
            "value": 59488.91908167032
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_audit_logging/time_range_queries_10000",
            "unit": "ns",
            "value": 39047.236044277066
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_caching/cache_expiration_check_5000",
            "unit": "ns",
            "value": 531477.3379598209
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_caching/cache_hits_10000",
            "unit": "ns",
            "value": 1264057.3927304307
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_caching/cache_population_5000",
            "unit": "ns",
            "value": 1322520.185152348
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_caching/lru_eviction_simulation_10000",
            "unit": "ns",
            "value": 85865605.92
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/bulk_encrypt_1000_secrets",
            "unit": "ns",
            "value": 240770.407595319
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/key_derivation_iterations",
            "unit": "ns",
            "value": 80231.47386686127
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/key_rotation_500_secrets",
            "unit": "ns",
            "value": 47118.507499859166
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/xor_encrypt/1024",
            "unit": "ns",
            "value": 3838.1780689899256
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/xor_encrypt/256",
            "unit": "ns",
            "value": 970.3369580590844
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/xor_encrypt/4096",
            "unit": "ns",
            "value": 15363.92137386194
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_encryption/xor_encrypt/64",
            "unit": "ns",
            "value": 253.42045921351215
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_key_management/key_generation_1000",
            "unit": "ns",
            "value": 96485.40801749531
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_key_management/key_hierarchy_1000",
            "unit": "ns",
            "value": 239731.6829126209
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_key_management/key_lookup_10000",
            "unit": "ns",
            "value": 658694.0049677229
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_key_management/key_rotation_tracking_500",
            "unit": "ns",
            "value": 50177.051585375804
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_keyring_operations/lock_unlock_cycles_1000",
            "unit": "ns",
            "value": 69518.81227580908
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_keyring_operations/multi_service_lookup_10000",
            "unit": "ns",
            "value": 94244.1011072414
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_keyring_operations/retrieve_credentials_5000",
            "unit": "ns",
            "value": 821097.2063180462
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_keyring_operations/store_credentials_5000",
            "unit": "ns",
            "value": 1638094.908614745
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_secret_storage/delete_secrets_5000",
            "unit": "ns",
            "value": 1387082.1281383112
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_secret_storage/list_filter_20000",
            "unit": "ns",
            "value": 1009038.3230638348
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_secret_storage/retrieve_secrets_10000",
            "unit": "ns",
            "value": 1085633.308330669
          },
          {
            "name": "astraweave-secrets::secrets_adversarial_secret_storage/store_secrets_10000",
            "unit": "ns",
            "value": 2676612.5326315784
          },
          {
            "name": "astraweave-physics::bone_def_default",
            "unit": "ns",
            "value": 10.099374786136229
          },
          {
            "name": "astraweave-physics::bone_shape_volume/box",
            "unit": "ns",
            "value": 0.634107350638413
          },
          {
            "name": "astraweave-physics::bone_shape_volume/capsule",
            "unit": "ns",
            "value": 0.625340926524748
          },
          {
            "name": "astraweave-physics::bone_shape_volume/sphere",
            "unit": "ns",
            "value": 0.6254714755490915
          },
          {
            "name": "astraweave-physics::character_full_tick",
            "unit": "ns",
            "value": 4846.122959247411
          },
          {
            "name": "astraweave-physics::character_move_batch/1",
            "unit": "ns",
            "value": 144.7837069156601
          },
          {
            "name": "astraweave-physics::character_move_batch/10",
            "unit": "ns",
            "value": 1471.4650748494291
          },
          {
            "name": "astraweave-physics::character_move_batch/100",
            "unit": "ns",
            "value": 25018.876090309976
          },
          {
            "name": "astraweave-physics::character_move_batch/50",
            "unit": "ns",
            "value": 9762.318733635579
          },
          {
            "name": "astraweave-physics::character_move_diagonal",
            "unit": "ns",
            "value": 102.69871498558769
          },
          {
            "name": "astraweave-physics::character_move_straight",
            "unit": "ns",
            "value": 101.68943808189505
          },
          {
            "name": "astraweave-physics::character_move_with_obstacles",
            "unit": "ns",
            "value": 106.92809300481738
          },
          {
            "name": "astraweave-physics::character_step_climbing",
            "unit": "ns",
            "value": 92.426724133132
          },
          {
            "name": "astraweave-physics::character_transform_lookup",
            "unit": "ns",
            "value": 18.91408112136238
          },
          {
            "name": "astraweave-physics::cloth_creation/16",
            "unit": "ns",
            "value": 3890.5418535067174
          },
          {
            "name": "astraweave-physics::cloth_creation/32",
            "unit": "ns",
            "value": 13217.277676759655
          },
          {
            "name": "astraweave-physics::cloth_creation/4",
            "unit": "ns",
            "value": 430.4737281438149
          },
          {
            "name": "astraweave-physics::cloth_creation/8",
            "unit": "ns",
            "value": 1000.0123793099656
          },
          {
            "name": "astraweave-physics::cloth_multiple/1",
            "unit": "ns",
            "value": 14703.800044432039
          },
          {
            "name": "astraweave-physics::cloth_multiple/10",
            "unit": "ns",
            "value": 143817.97027500364
          },
          {
            "name": "astraweave-physics::cloth_multiple/20",
            "unit": "ns",
            "value": 287812.7163544001
          },
          {
            "name": "astraweave-physics::cloth_multiple/5",
            "unit": "ns",
            "value": 72209.24577441945
          },
          {
            "name": "astraweave-physics::cloth_stiffness/0.3",
            "unit": "ns",
            "value": 64335.895436216546
          },
          {
            "name": "astraweave-physics::cloth_stiffness/0.6",
            "unit": "ns",
            "value": 64335.139585034296
          },
          {
            "name": "astraweave-physics::cloth_stiffness/0.9",
            "unit": "ns",
            "value": 64339.425624172414
          },
          {
            "name": "astraweave-physics::cloth_update_16x16",
            "unit": "ns",
            "value": 64834.70731949414
          },
          {
            "name": "astraweave-physics::cloth_update_32x32",
            "unit": "ns",
            "value": 272559.82335497375
          },
          {
            "name": "astraweave-physics::cloth_update_8x8",
            "unit": "ns",
            "value": 14373.772084984208
          },
          {
            "name": "astraweave-physics::destruction_creation/16",
            "unit": "ns",
            "value": 269.32021576407124
          },
          {
            "name": "astraweave-physics::destruction_creation/32",
            "unit": "ns",
            "value": 315.48272465993034
          },
          {
            "name": "astraweave-physics::destruction_creation/4",
            "unit": "ns",
            "value": 191.6155310937938
          },
          {
            "name": "astraweave-physics::destruction_creation/8",
            "unit": "ns",
            "value": 199.56482018639585
          },
          {
            "name": "astraweave-physics::destruction_damage/10",
            "unit": "ns",
            "value": 140.24530366276468
          },
          {
            "name": "astraweave-physics::destruction_damage/100",
            "unit": "ns",
            "value": 1427.0291295689503
          },
          {
            "name": "astraweave-physics::destruction_damage/50",
            "unit": "ns",
            "value": 711.5347997560733
          },
          {
            "name": "astraweave-physics::destruction_destroy_single",
            "unit": "ns",
            "value": 1074.0597035111623
          },
          {
            "name": "astraweave-physics::destruction_patterns/large_64",
            "unit": "ns",
            "value": 264.54104371649845
          },
          {
            "name": "astraweave-physics::destruction_patterns/medium_16",
            "unit": "ns",
            "value": 159.32941553174513
          },
          {
            "name": "astraweave-physics::destruction_patterns/small_4",
            "unit": "ns",
            "value": 147.4779945679058
          },
          {
            "name": "astraweave-physics::destruction_scale/100",
            "unit": "ns",
            "value": 3027.5772401861905
          },
          {
            "name": "astraweave-physics::destruction_scale/1000",
            "unit": "ns",
            "value": 38691.145875498994
          },
          {
            "name": "astraweave-physics::destruction_scale/500",
            "unit": "ns",
            "value": 17335.778328490152
          },
          {
            "name": "astraweave-physics::destruction_update/10",
            "unit": "ns",
            "value": 28.190681800265175
          },
          {
            "name": "astraweave-physics::destruction_update/100",
            "unit": "ns",
            "value": 214.75838071396876
          },
          {
            "name": "astraweave-physics::destruction_update/50",
            "unit": "ns",
            "value": 108.79652733208312
          },
          {
            "name": "astraweave-physics::engine_torque_curve",
            "unit": "ns",
            "value": 105.31764797446996
          },
          {
            "name": "astraweave-physics::friction_curve/gravel",
            "unit": "ns",
            "value": 222.99467984889972
          },
          {
            "name": "astraweave-physics::friction_curve/ice",
            "unit": "ns",
            "value": 193.3979678032173
          },
          {
            "name": "astraweave-physics::friction_curve/mud",
            "unit": "ns",
            "value": 238.37380674997232
          },
          {
            "name": "astraweave-physics::friction_curve/tarmac",
            "unit": "ns",
            "value": 202.97530567888234
          },
          {
            "name": "astraweave-physics::ragdoll_add_ball_bone",
            "unit": "ns",
            "value": 56.47313512331687
          },
          {
            "name": "astraweave-physics::ragdoll_add_hinge_bone",
            "unit": "ns",
            "value": 59.96091490845615
          },
          {
            "name": "astraweave-physics::ragdoll_builder/10",
            "unit": "ns",
            "value": 938.0214163126241
          },
          {
            "name": "astraweave-physics::ragdoll_builder/20",
            "unit": "ns",
            "value": 2052.6601392433727
          },
          {
            "name": "astraweave-physics::ragdoll_builder/30",
            "unit": "ns",
            "value": 3228.9591018832302
          },
          {
            "name": "astraweave-physics::ragdoll_builder/5",
            "unit": "ns",
            "value": 439.2903097242899
          },
          {
            "name": "astraweave-physics::ragdoll_config_default",
            "unit": "ns",
            "value": 3.735984601730882
          },
          {
            "name": "astraweave-physics::ragdoll_config_variations/floppy",
            "unit": "ns",
            "value": 110.27500129199794
          },
          {
            "name": "astraweave-physics::ragdoll_config_variations/heavy",
            "unit": "ns",
            "value": 110.14567110084553
          },
          {
            "name": "astraweave-physics::ragdoll_config_variations/stiff",
            "unit": "ns",
            "value": 109.8033686804906
          },
          {
            "name": "astraweave-physics::ragdoll_humanoid_skeleton",
            "unit": "ns",
            "value": 978.6056282915374
          },
          {
            "name": "astraweave-physics::raycast_batch_8_rays",
            "unit": "ns",
            "value": 2272.8584085906523
          },
          {
            "name": "astraweave-physics::raycast_empty_scene",
            "unit": "ns",
            "value": 13.098627325038148
          },
          {
            "name": "astraweave-physics::raycast_ground_plane",
            "unit": "ns",
            "value": 186.34588070899727
          },
          {
            "name": "astraweave-physics::raycast_normal_retrieval/with_normal",
            "unit": "ns",
            "value": 184.8394239211895
          },
          {
            "name": "astraweave-physics::raycast_normal_retrieval/without_normal",
            "unit": "ns",
            "value": 139.6340299641059
          },
          {
            "name": "astraweave-physics::raycast_obstacle_density/0",
            "unit": "ns",
            "value": 98.8295440148572
          },
          {
            "name": "astraweave-physics::raycast_obstacle_density/10",
            "unit": "ns",
            "value": 182.43484573778593
          },
          {
            "name": "astraweave-physics::raycast_obstacle_density/100",
            "unit": "ns",
            "value": 256.6341118670184
          },
          {
            "name": "astraweave-physics::raycast_obstacle_density/50",
            "unit": "ns",
            "value": 266.13852185829523
          },
          {
            "name": "astraweave-physics::rigid_body_batch_step/1",
            "unit": "ns",
            "value": 921.0698678441738
          },
          {
            "name": "astraweave-physics::rigid_body_batch_step/10",
            "unit": "ns",
            "value": 2386.482500342791
          },
          {
            "name": "astraweave-physics::rigid_body_batch_step/100",
            "unit": "ns",
            "value": 13524.963780812051
          },
          {
            "name": "astraweave-physics::rigid_body_batch_step/200",
            "unit": "ns",
            "value": 21486.352435823428
          },
          {
            "name": "astraweave-physics::rigid_body_batch_step/50",
            "unit": "ns",
            "value": 7165.459747834717
          },
          {
            "name": "astraweave-physics::rigid_body_creation",
            "unit": "ns",
            "value": 4053.1505187781454
          },
          {
            "name": "astraweave-physics::rigid_body_destructible_creation",
            "unit": "ns",
            "value": 4014.00507011741
          },
          {
            "name": "astraweave-physics::rigid_body_ground_creation",
            "unit": "ns",
            "value": 3958.9166051302
          },
          {
            "name": "astraweave-physics::rigid_body_mixed_simulation",
            "unit": "ns",
            "value": 7115.554426688339
          },
          {
            "name": "astraweave-physics::rigid_body_single_step",
            "unit": "ns",
            "value": 1321.5093644944218
          },
          {
            "name": "astraweave-physics::rigid_body_stacked_simulation",
            "unit": "ns",
            "value": 4341.575610664294
          },
          {
            "name": "astraweave-physics::rigid_body_transform_lookup",
            "unit": "ns",
            "value": 20.3728500278517
          },
          {
            "name": "astraweave-physics::rigid_body_trimesh_creation",
            "unit": "ns",
            "value": 4230.875740626539
          },
          {
            "name": "astraweave-physics::transmission_effective_ratio",
            "unit": "ns",
            "value": 7.5462229112756605
          },
          {
            "name": "astraweave-physics::vehicle_creation",
            "unit": "ns",
            "value": 134723.23606228887
          },
          {
            "name": "astraweave-physics::vehicle_multiple/1",
            "unit": "ns",
            "value": 105.80979445358021
          },
          {
            "name": "astraweave-physics::vehicle_multiple/10",
            "unit": "ns",
            "value": 1100.6746700675114
          },
          {
            "name": "astraweave-physics::vehicle_multiple/20",
            "unit": "ns",
            "value": 2441.135360170751
          },
          {
            "name": "astraweave-physics::vehicle_multiple/5",
            "unit": "ns",
            "value": 544.7238804501023
          },
          {
            "name": "astraweave-physics::vehicle_shifting",
            "unit": "ns",
            "value": 3.1136388743422754
          },
          {
            "name": "astraweave-physics::vehicle_update_single",
            "unit": "ns",
            "value": 105.4649908863354
          },
          {
            "name": "astraweave-physics::vehicle_update_with_input",
            "unit": "ns",
            "value": 163.46694177287452
          },
          {
            "name": "astraweave-npc::npc_adversarial_behavior_systems/behavior_tree_eval_1000",
            "unit": "ns",
            "value": 2747.5492617425125
          },
          {
            "name": "astraweave-npc::npc_adversarial_behavior_systems/goap_planning_200",
            "unit": "ns",
            "value": 125784.25700564968
          },
          {
            "name": "astraweave-npc::npc_adversarial_behavior_systems/state_transitions_5000",
            "unit": "ns",
            "value": 20461.795651620763
          },
          {
            "name": "astraweave-npc::npc_adversarial_behavior_systems/utility_scoring_500",
            "unit": "ns",
            "value": 2777.4809087967697
          },
          {
            "name": "astraweave-npc::npc_adversarial_dialogue_systems/dialogue_tree_traversal_500",
            "unit": "ns",
            "value": 14272.529104276055
          },
          {
            "name": "astraweave-npc::npc_adversarial_dialogue_systems/emotion_blending_1000",
            "unit": "ns",
            "value": 0.349108691748968
          },
          {
            "name": "astraweave-npc::npc_adversarial_dialogue_systems/keyword_matching_2000",
            "unit": "ns",
            "value": 311091.81089497084
          },
          {
            "name": "astraweave-npc::npc_adversarial_dialogue_systems/response_selection_500",
            "unit": "ns",
            "value": 200534.70766797208
          },
          {
            "name": "astraweave-npc::npc_adversarial_llm_integration/context_building_200",
            "unit": "ns",
            "value": 106975.42625753109
          },
          {
            "name": "astraweave-npc::npc_adversarial_llm_integration/conversation_history_500",
            "unit": "ns",
            "value": 178149.71779083053
          },
          {
            "name": "astraweave-npc::npc_adversarial_llm_integration/prompt_formatting_500",
            "unit": "ns",
            "value": 41971.339307000766
          },
          {
            "name": "astraweave-npc::npc_adversarial_llm_integration/response_parsing_1000",
            "unit": "ns",
            "value": 382244.0472878656
          },
          {
            "name": "astraweave-npc::npc_adversarial_profile_management/personality_compatibility_1000",
            "unit": "ns",
            "value": 9427.652638013727
          },
          {
            "name": "astraweave-npc::npc_adversarial_profile_management/profile_creation_500",
            "unit": "ns",
            "value": 319954.813977148
          },
          {
            "name": "astraweave-npc::npc_adversarial_profile_management/relationship_updates_5000",
            "unit": "ns",
            "value": 66409.25050880508
          },
          {
            "name": "astraweave-npc::npc_adversarial_profile_management/schedule_lookup_10000",
            "unit": "ns",
            "value": 76496.93783937984
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/action_queue_processing_1000",
            "unit": "ns",
            "value": 34.59366667966675
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/lod_management_2000",
            "unit": "ns",
            "value": 5602.298247079965
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/path_following_500",
            "unit": "ns",
            "value": 183.53448062592423
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/tick_update/1000",
            "unit": "ns",
            "value": 1646.173732670793
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/tick_update/2000",
            "unit": "ns",
            "value": 3347.5679940270534
          },
          {
            "name": "astraweave-npc::npc_adversarial_runtime_systems/tick_update/500",
            "unit": "ns",
            "value": 832.5317598102682
          },
          {
            "name": "astraweave-npc::npc_adversarial_sense_systems/memory_decay_2000",
            "unit": "ns",
            "value": 142.39349561416392
          },
          {
            "name": "astraweave-npc::npc_adversarial_sense_systems/sound_propagation_1000",
            "unit": "ns",
            "value": 80652.53434598449
          },
          {
            "name": "astraweave-npc::npc_adversarial_sense_systems/threat_assessment_500",
            "unit": "ns",
            "value": 1227.3013846980432
          },
          {
            "name": "astraweave-npc::npc_adversarial_sense_systems/vision_cone_5000",
            "unit": "ns",
            "value": 877664.1964064597
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/highly_connected_graph_100",
            "unit": "ns",
            "value": 8385.294590083386
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/layered_material_5_layers",
            "unit": "ns",
            "value": 299.979692103565
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/procedural_material/100",
            "unit": "ns",
            "value": 925.3287424207824
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/procedural_material/200",
            "unit": "ns",
            "value": 1386.0747282728905
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/procedural_material/50",
            "unit": "ns",
            "value": 648.0247938908094
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_construction/simple_pbr_material",
            "unit": "ns",
            "value": 52.424814390757476
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_optimization/constant_folding_100",
            "unit": "ns",
            "value": 186.45588149537602
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_optimization/cse_detection_500",
            "unit": "ns",
            "value": 15166.179070386628
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_optimization/dead_code_elimination_200",
            "unit": "ns",
            "value": 1744.9253074349429
          },
          {
            "name": "astraweave-materials::materials_adversarial_graph_optimization/gpu_reorder_200",
            "unit": "ns",
            "value": 612.0761818109395
          },
          {
            "name": "astraweave-materials::materials_adversarial_material_instances/create_instances_100",
            "unit": "ns",
            "value": 29846.730994680667
          },
          {
            "name": "astraweave-materials::materials_adversarial_material_instances/parameter_lookup_10000",
            "unit": "ns",
            "value": 172061.8107555494
          },
          {
            "name": "astraweave-materials::materials_adversarial_material_instances/sort_by_shader_1000",
            "unit": "ns",
            "value": 32855.02153842319
          },
          {
            "name": "astraweave-materials::materials_adversarial_material_instances/update_parameters_500",
            "unit": "ns",
            "value": 17905.85694580646
          },
          {
            "name": "astraweave-materials::materials_adversarial_node_evaluation/constant_evaluation_1000",
            "unit": "ns",
            "value": 1056.1130509284426
          },
          {
            "name": "astraweave-materials::materials_adversarial_node_evaluation/fresnel_evaluation_500",
            "unit": "ns",
            "value": 0.6571688765077599
          },
          {
            "name": "astraweave-materials::materials_adversarial_node_evaluation/math_chain_evaluation_500",
            "unit": "ns",
            "value": 1171.3030621179785
          },
          {
            "name": "astraweave-materials::materials_adversarial_node_evaluation/trig_evaluation_1000",
            "unit": "ns",
            "value": 17100.148748112424
          },
          {
            "name": "astraweave-materials::materials_adversarial_node_evaluation/vector_operations_1000",
            "unit": "ns",
            "value": 0.593130868121937
          },
          {
            "name": "astraweave-materials::materials_adversarial_texture_binding/binding_layout_generation",
            "unit": "ns",
            "value": 20332.219959608414
          },
          {
            "name": "astraweave-materials::materials_adversarial_texture_binding/sampler_deduplication_200",
            "unit": "ns",
            "value": 672.6666546565747
          },
          {
            "name": "astraweave-materials::materials_adversarial_texture_binding/slot_assignment_100",
            "unit": "ns",
            "value": 1338.7520497781659
          },
          {
            "name": "astraweave-materials::materials_adversarial_texture_binding/texture_array_packing_50",
            "unit": "ns",
            "value": 1348.5135451703893
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/batch_compile_50_materials",
            "unit": "ns",
            "value": 19595.772859200206
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/compile_10_textures",
            "unit": "ns",
            "value": 3622.1960552376427
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/compile_complex_material/100",
            "unit": "ns",
            "value": 10995.820156628475
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/compile_complex_material/200",
            "unit": "ns",
            "value": 21845.476776447293
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/compile_complex_material/50",
            "unit": "ns",
            "value": 5459.897961328237
          },
          {
            "name": "astraweave-materials::materials_adversarial_wgsl_compilation/compile_simple_material",
            "unit": "ns",
            "value": 342.79319672291894
          },
          {
            "name": "astraweave-gameplay::attack_scaling/1",
            "unit": "ns",
            "value": 55.12094247041003
          },
          {
            "name": "astraweave-gameplay::attack_scaling/10",
            "unit": "ns",
            "value": 715.2669346279897
          },
          {
            "name": "astraweave-gameplay::attack_scaling/100",
            "unit": "ns",
            "value": 20649.917508576058
          },
          {
            "name": "astraweave-gameplay::attack_scaling/25",
            "unit": "ns",
            "value": 2206.2034814410554
          },
          {
            "name": "astraweave-gameplay::attack_scaling/5",
            "unit": "ns",
            "value": 251.71776856814458
          },
          {
            "name": "astraweave-gameplay::attack_scaling/50",
            "unit": "ns",
            "value": 6396.124476071284
          },
          {
            "name": "astraweave-gameplay::large_battle/100v100_battle",
            "unit": "ns",
            "value": 28751.545678984978
          },
          {
            "name": "astraweave-gameplay::multi_attacker_scenario/100_attackers_20_targets",
            "unit": "ns",
            "value": 12897.238680992581
          },
          {
            "name": "astraweave-gameplay::multi_attacker_scenario/10_attackers_5_targets",
            "unit": "ns",
            "value": 468.1513062120734
          },
          {
            "name": "astraweave-gameplay::multi_attacker_scenario/50_attackers_10_targets",
            "unit": "ns",
            "value": 5751.918644898176
          },
          {
            "name": "astraweave-gameplay::single_attack/5_targets_no_defense",
            "unit": "ns",
            "value": 30.423228391123807
          },
          {
            "name": "astraweave-gameplay::single_attack_iframe/5_targets_50%_iframe",
            "unit": "ns",
            "value": 15.181122188123878
          },
          {
            "name": "astraweave-gameplay::single_attack_parry/5_targets_50%_parry",
            "unit": "ns",
            "value": 15.090024935621662
          },
          {
            "name": "astraweave-combat::resolve_hit",
            "unit": "ns",
            "value": 216.85602613442646
          },
          {
            "name": "astraweave-combat::resolve_hit_batch_10k",
            "unit": "ns",
            "value": 2069614.3172000009
          },
          {
            "name": "astraweave-scene::AABB/center_computation",
            "unit": "ns",
            "value": 1.2460348443387015
          },
          {
            "name": "astraweave-scene::AABB/contains_point_inside",
            "unit": "ns",
            "value": 0.6245530399987413
          },
          {
            "name": "astraweave-scene::AABB/contains_point_outside",
            "unit": "ns",
            "value": 0.6225034358412147
          },
          {
            "name": "astraweave-scene::AABB/from_center_half_extents",
            "unit": "ns",
            "value": 6.6861750211896664
          },
          {
            "name": "astraweave-scene::AABB/half_extents_computation",
            "unit": "ns",
            "value": 1.2469251805546944
          },
          {
            "name": "astraweave-scene::AABB/intersects_overlapping",
            "unit": "ns",
            "value": 0.6223465581445089
          },
          {
            "name": "astraweave-scene::AABB/intersects_separate",
            "unit": "ns",
            "value": 0.6221303975392248
          },
          {
            "name": "astraweave-scene::AABB/new_creation",
            "unit": "ns",
            "value": 7.158896710032804
          },
          {
            "name": "astraweave-scene::AABB/overlapping_cells/1x1x1",
            "unit": "ns",
            "value": 86.08291185040589
          },
          {
            "name": "astraweave-scene::AABB/overlapping_cells/4x4x4",
            "unit": "ns",
            "value": 431.05307253521465
          },
          {
            "name": "astraweave-scene::AABB/overlapping_cells/8x8x8",
            "unit": "ns",
            "value": 1693.5578789998838
          },
          {
            "name": "astraweave-scene::CellEntities/add_entities_batch/10",
            "unit": "ns",
            "value": 120.00514227127847
          },
          {
            "name": "astraweave-scene::CellEntities/add_entities_batch/100",
            "unit": "ns",
            "value": 1373.3917381057927
          },
          {
            "name": "astraweave-scene::CellEntities/add_entities_batch/50",
            "unit": "ns",
            "value": 485.76989427970375
          },
          {
            "name": "astraweave-scene::CellEntities/add_entity_dedup",
            "unit": "ns",
            "value": 0.9345342678960594
          },
          {
            "name": "astraweave-scene::CellEntities/add_entity_first",
            "unit": "ns",
            "value": 1.246923833306088
          },
          {
            "name": "astraweave-scene::CellEntities/creation",
            "unit": "ns",
            "value": 6.224447331057409
          },
          {
            "name": "astraweave-scene::CellEntities/remove_entity_absent",
            "unit": "ns",
            "value": 6.542545742627072
          },
          {
            "name": "astraweave-scene::CellEntities/remove_entity_present",
            "unit": "ns",
            "value": 11.50655557324603
          },
          {
            "name": "astraweave-scene::FrustumCulling/batch_cull_aabbs/10",
            "unit": "ns",
            "value": 112.78918009350906
          },
          {
            "name": "astraweave-scene::FrustumCulling/batch_cull_aabbs/100",
            "unit": "ns",
            "value": 452.58307854152196
          },
          {
            "name": "astraweave-scene::FrustumCulling/batch_cull_aabbs/50",
            "unit": "ns",
            "value": 304.2823476823198
          },
          {
            "name": "astraweave-scene::FrustumCulling/batch_cull_aabbs/500",
            "unit": "ns",
            "value": 1662.1295571166527
          },
          {
            "name": "astraweave-scene::FrustumCulling/from_orthographic_matrix",
            "unit": "ns",
            "value": 36.37609362992635
          },
          {
            "name": "astraweave-scene::FrustumCulling/from_perspective_matrix",
            "unit": "ns",
            "value": 36.39475649246959
          },
          {
            "name": "astraweave-scene::FrustumCulling/intersects_aabb_inside",
            "unit": "ns",
            "value": 0.6233318414204979
          },
          {
            "name": "astraweave-scene::FrustumCulling/intersects_aabb_outside",
            "unit": "ns",
            "value": 0.6225828658108538
          },
          {
            "name": "astraweave-scene::FrustumCulling/intersects_aabb_partial",
            "unit": "ns",
            "value": 0.6228238416349443
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/can_allocate_no",
            "unit": "ns",
            "value": 0.6235868721050493
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/can_allocate_yes",
            "unit": "ns",
            "value": 0.6224892479060771
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/creation_500mb",
            "unit": "ns",
            "value": 5.610328645159567
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/find_furthest_cell_100",
            "unit": "ns",
            "value": 360.99694922339154
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/find_furthest_cell_25",
            "unit": "ns",
            "value": 73.56877440130314
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/get_or_create_cell",
            "unit": "ns",
            "value": 87.94606070057468
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/stats_computation",
            "unit": "ns",
            "value": 6.5363691781045254
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/unload_cell",
            "unit": "ns",
            "value": 82.20208299410861
          },
          {
            "name": "astraweave-scene::GpuResourceBudget/update_usage_25_cells",
            "unit": "ns",
            "value": 24.524946596913566
          },
          {
            "name": "astraweave-scene::GridCoord/from_world_pos_negative",
            "unit": "ns",
            "value": 5.909688927503685
          },
          {
            "name": "astraweave-scene::GridCoord/from_world_pos_origin",
            "unit": "ns",
            "value": 5.909688132174089
          },
          {
            "name": "astraweave-scene::GridCoord/from_world_pos_positive",
            "unit": "ns",
            "value": 5.91008654997492
          },
          {
            "name": "astraweave-scene::GridCoord/hash_lookup",
            "unit": "ns",
            "value": 20.06190226309091
          },
          {
            "name": "astraweave-scene::GridCoord/manhattan_distance",
            "unit": "ns",
            "value": 0.6228249172005839
          },
          {
            "name": "astraweave-scene::GridCoord/neighbors_2d_8",
            "unit": "ns",
            "value": 26.270330661986534
          },
          {
            "name": "astraweave-scene::GridCoord/neighbors_3d_26",
            "unit": "ns",
            "value": 56.78553078955237
          },
          {
            "name": "astraweave-scene::GridCoord/new_creation",
            "unit": "ns",
            "value": 1.2463179732870422
          },
          {
            "name": "astraweave-scene::GridCoord/to_world_center",
            "unit": "ns",
            "value": 1.2460446106797958
          },
          {
            "name": "astraweave-scene::LRUCache/contains_absent",
            "unit": "ns",
            "value": 4.474772977921429
          },
          {
            "name": "astraweave-scene::LRUCache/contains_present",
            "unit": "ns",
            "value": 2.380401782397474
          },
          {
            "name": "astraweave-scene::LRUCache/creation/100",
            "unit": "ns",
            "value": 38.461220400447836
          },
          {
            "name": "astraweave-scene::LRUCache/creation/25",
            "unit": "ns",
            "value": 10.606128443464051
          },
          {
            "name": "astraweave-scene::LRUCache/creation/5",
            "unit": "ns",
            "value": 10.59480882082242
          },
          {
            "name": "astraweave-scene::LRUCache/lru_retrieval",
            "unit": "ns",
            "value": 1.8664132830861613
          },
          {
            "name": "astraweave-scene::LRUCache/remove_existing",
            "unit": "ns",
            "value": 23.61893126868807
          },
          {
            "name": "astraweave-scene::LRUCache/touch_existing_entry",
            "unit": "ns",
            "value": 10.903351078491083
          },
          {
            "name": "astraweave-scene::LRUCache/touch_new_entry",
            "unit": "ns",
            "value": 11.564572643969441
          },
          {
            "name": "astraweave-scene::LRUCache/touch_with_eviction",
            "unit": "ns",
            "value": 7.6127032087618
          },
          {
            "name": "astraweave-scene::SceneGraph/node_creation_str",
            "unit": "ns",
            "value": 21.131725800563736
          },
          {
            "name": "astraweave-scene::SceneGraph/node_creation_string",
            "unit": "ns",
            "value": 21.139206293117123
          },
          {
            "name": "astraweave-scene::SceneGraph/scene_creation",
            "unit": "ns",
            "value": 20.949978532450682
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_linear_depth/1",
            "unit": "ns",
            "value": 17.533550360285677
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_linear_depth/10",
            "unit": "ns",
            "value": 138.2054923367718
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_linear_depth/20",
            "unit": "ns",
            "value": 277.10554283809086
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_linear_depth/5",
            "unit": "ns",
            "value": 68.40255845836933
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_tree_3x3x3",
            "unit": "ns",
            "value": 275.2659924195574
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_wide_children/10",
            "unit": "ns",
            "value": 52.237005044119215
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_wide_children/2",
            "unit": "ns",
            "value": 19.653700241784293
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_wide_children/20",
            "unit": "ns",
            "value": 95.17453130549285
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_wide_children/5",
            "unit": "ns",
            "value": 33.16670855395727
          },
          {
            "name": "astraweave-scene::SceneGraph/traverse_with_transform_extraction",
            "unit": "ns",
            "value": 85.53249820779106
          },
          {
            "name": "astraweave-scene::SpatialQueries/entity_cell_lookup",
            "unit": "ns",
            "value": 13.813376300406846
          },
          {
            "name": "astraweave-scene::SpatialQueries/query_entities_5_cells",
            "unit": "ns",
            "value": 274.872727243768
          },
          {
            "name": "astraweave-scene::SpatialQueries/query_radius_entities",
            "unit": "ns",
            "value": 1333.2408394826698
          },
          {
            "name": "astraweave-scene::Transform/default_creation",
            "unit": "ns",
            "value": 7.470439888496867
          },
          {
            "name": "astraweave-scene::Transform/from_trs_creation",
            "unit": "ns",
            "value": 7.157906154748451
          },
          {
            "name": "astraweave-scene::Transform/matrix_chain_2",
            "unit": "ns",
            "value": 2.4944036852502696
          },
          {
            "name": "astraweave-scene::Transform/matrix_chain_5",
            "unit": "ns",
            "value": 52.15550911252895
          },
          {
            "name": "astraweave-scene::Transform/matrix_decompose_trs",
            "unit": "ns",
            "value": 7.15572344713083
          },
          {
            "name": "astraweave-scene::Transform/matrix_full_trs",
            "unit": "ns",
            "value": 2.492861621162474
          },
          {
            "name": "astraweave-scene::Transform/matrix_identity",
            "unit": "ns",
            "value": 2.4917413039144836
          },
          {
            "name": "astraweave-scene::Transform/matrix_translation_only",
            "unit": "ns",
            "value": 2.4930913446756864
          },
          {
            "name": "astraweave-scene::WorldPartition/assign_entity_single",
            "unit": "ns",
            "value": 337.154615529884
          },
          {
            "name": "astraweave-scene::WorldPartition/cells_in_radius/100m",
            "unit": "ns",
            "value": 57.07252373898099
          },
          {
            "name": "astraweave-scene::WorldPartition/cells_in_radius/300m",
            "unit": "ns",
            "value": 300.0080325393843
          },
          {
            "name": "astraweave-scene::WorldPartition/cells_in_radius/500m",
            "unit": "ns",
            "value": 535.4632855567041
          },
          {
            "name": "astraweave-scene::WorldPartition/cells_in_radius_populated",
            "unit": "ns",
            "value": 283.81091043759835
          },
          {
            "name": "astraweave-scene::WorldPartition/creation_default",
            "unit": "ns",
            "value": 6.6456372878471
          },
          {
            "name": "astraweave-scene::WorldPartition/get_or_create_cell_existing",
            "unit": "ns",
            "value": 22.79062887104937
          },
          {
            "name": "astraweave-scene::WorldPartition/get_or_create_cell_new",
            "unit": "ns",
            "value": 68.73980997934048
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/batch_padding_1000",
            "unit": "ns",
            "value": 22646.089441373948
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/dynamic_batch_sizing_2000",
            "unit": "ns",
            "value": 1481.4354285428478
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/priority_sorting_5000",
            "unit": "ns",
            "value": 4794.191319911511
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/request_batching/16",
            "unit": "ns",
            "value": 54536.731203608055
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/request_batching/32",
            "unit": "ns",
            "value": 49877.46027366584
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_batch_inference/request_batching/64",
            "unit": "ns",
            "value": 46366.797965695376
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_compression/embedding_quantization_1000",
            "unit": "ns",
            "value": 2332318.731363637
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_compression/kv_cache_compression_500",
            "unit": "ns",
            "value": 94943.79067326261
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_compression/sparse_attention_1000",
            "unit": "ns",
            "value": 156862.73896087238
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_compression/token_compression_10000",
            "unit": "ns",
            "value": 3968331.793846154
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/health_checking_5000",
            "unit": "ns",
            "value": 69886.32053854836
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/load_rebalancing_1000",
            "unit": "ns",
            "value": 1056.3389542128682
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/request_routing_10000",
            "unit": "ns",
            "value": 102507.83946237061
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/worker_selection/LatencyAware",
            "unit": "ns",
            "value": 86058.32504414156
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/worker_selection/LeastLoaded",
            "unit": "ns",
            "value": 87008.81233760827
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/worker_selection/RoundRobin",
            "unit": "ns",
            "value": 23054.96070506648
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_load_balancing/worker_selection/WeightedRandom",
            "unit": "ns",
            "value": 51417.293655423615
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_model_optimization/activation_checkpointing_500",
            "unit": "ns",
            "value": 900.7456036451133
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_model_optimization/distillation_prep_1000",
            "unit": "ns",
            "value": 496091805.15
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_model_optimization/gradient_accumulation_2000",
            "unit": "ns",
            "value": 235824.66285392683
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_model_optimization/weight_pruning_10000",
            "unit": "ns",
            "value": 5587.602959574956
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_prompt_caching/cache_eviction_5000",
            "unit": "ns",
            "value": 3707082.550000001
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_prompt_caching/cache_lookup_10000",
            "unit": "ns",
            "value": 284670.33609409275
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_prompt_caching/cache_statistics_1000",
            "unit": "ns",
            "value": 1953.3024153841206
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_prompt_caching/prefix_matching_5000",
            "unit": "ns",
            "value": 5120384.760999998
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_token_optimization/budget_allocation_500",
            "unit": "ns",
            "value": 4748.2874440820815
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_token_optimization/context_packing_1000",
            "unit": "ns",
            "value": 9345.241994759266
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_token_optimization/token_merging_2000",
            "unit": "ns",
            "value": 103300.32191375621
          },
          {
            "name": "astraweave-optimization::optimization_adversarial_token_optimization/tokenization_5000",
            "unit": "ns",
            "value": 1127862.3064471115
          },
          {
            "name": "astraweave-director::director_adversarial_boss_planning/enrage_calculation_1000",
            "unit": "ns",
            "value": 1642.7231630064075
          },
          {
            "name": "astraweave-director::director_adversarial_boss_planning/operation_selection_500",
            "unit": "ns",
            "value": 37247.9787048858
          },
          {
            "name": "astraweave-director::director_adversarial_boss_planning/phase_determination_1000",
            "unit": "ns",
            "value": 4939.810664218746
          },
          {
            "name": "astraweave-director::director_adversarial_boss_planning/plan_generation_100",
            "unit": "ns",
            "value": 31642.496975192164
          },
          {
            "name": "astraweave-director::director_adversarial_boss_planning/threat_assessment_200",
            "unit": "ns",
            "value": 719.9358675222552
          },
          {
            "name": "astraweave-director::director_adversarial_difficulty_scaling/dynamic_difficulty_adjustment",
            "unit": "ns",
            "value": 114.79005418965035
          },
          {
            "name": "astraweave-director::director_adversarial_difficulty_scaling/loot_quality_adjustment_200",
            "unit": "ns",
            "value": 7561.219719265755
          },
          {
            "name": "astraweave-director::director_adversarial_difficulty_scaling/skill_assessment_50",
            "unit": "ns",
            "value": 826.1092597990386
          },
          {
            "name": "astraweave-director::director_adversarial_difficulty_scaling/stat_scaling_1000",
            "unit": "ns",
            "value": 1897.9532338214685
          },
          {
            "name": "astraweave-director::director_adversarial_encounter_metrics/dps_calculation_100",
            "unit": "ns",
            "value": 20978.64727543427
          },
          {
            "name": "astraweave-director::director_adversarial_encounter_metrics/metrics_aggregation_1000",
            "unit": "ns",
            "value": 477.15929417713045
          },
          {
            "name": "astraweave-director::director_adversarial_encounter_metrics/performance_rating",
            "unit": "ns",
            "value": 116.28753593741727
          },
          {
            "name": "astraweave-director::director_adversarial_encounter_metrics/phase_timing_analysis",
            "unit": "ns",
            "value": 22.505113448922472
          },
          {
            "name": "astraweave-director::director_adversarial_llm_director/action_validation_200",
            "unit": "ns",
            "value": 1233.4741433842805
          },
          {
            "name": "astraweave-director::director_adversarial_llm_director/fallback_decision_100",
            "unit": "ns",
            "value": 5742.2097608344575
          },
          {
            "name": "astraweave-director::director_adversarial_llm_director/llm_context_building",
            "unit": "ns",
            "value": 588.8199907175623
          },
          {
            "name": "astraweave-director::director_adversarial_llm_director/llm_response_parsing_100",
            "unit": "ns",
            "value": 31814.772960440594
          },
          {
            "name": "astraweave-director::director_adversarial_minion_management/minion_coordination_100",
            "unit": "ns",
            "value": 3720.5170174502405
          },
          {
            "name": "astraweave-director::director_adversarial_minion_management/minion_death_tracking_500",
            "unit": "ns",
            "value": 7250.932251349715
          },
          {
            "name": "astraweave-director::director_adversarial_minion_management/minion_spawning_200",
            "unit": "ns",
            "value": 9821.925672021887
          },
          {
            "name": "astraweave-director::director_adversarial_minion_management/reinforcement_waves_20",
            "unit": "ns",
            "value": 2018.007245576476
          },
          {
            "name": "astraweave-director::director_adversarial_phase_transitions/ability_unlock_50",
            "unit": "ns",
            "value": 4442.671300538731
          },
          {
            "name": "astraweave-director::director_adversarial_phase_transitions/intermission_handling_50",
            "unit": "ns",
            "value": 295.2525882338114
          },
          {
            "name": "astraweave-director::director_adversarial_phase_transitions/transition_animation_scheduling",
            "unit": "ns",
            "value": 8704.877190372106
          },
          {
            "name": "astraweave-director::director_adversarial_phase_transitions/transition_detection_1000",
            "unit": "ns",
            "value": 8808.461978572694
          },
          {
            "name": "astraweave-nav::bake_100_triangles",
            "unit": "ns",
            "value": 42166.83050183186
          },
          {
            "name": "astraweave-nav::bake_10k_triangles",
            "unit": "ns",
            "value": 372797848.27
          },
          {
            "name": "astraweave-nav::bake_1k_triangles",
            "unit": "ns",
            "value": 3949855.4838461555
          },
          {
            "name": "astraweave-nav::baking_scaling/100",
            "unit": "ns",
            "value": 40866.791193263205
          },
          {
            "name": "astraweave-nav::baking_scaling/1000",
            "unit": "ns",
            "value": 3538457.887333335
          },
          {
            "name": "astraweave-nav::baking_scaling/10000",
            "unit": "ns",
            "value": 357388448.98
          },
          {
            "name": "astraweave-nav::baking_scaling/2000",
            "unit": "ns",
            "value": 13780525.73
          },
          {
            "name": "astraweave-nav::baking_scaling/500",
            "unit": "ns",
            "value": 783154.2392191854
          },
          {
            "name": "astraweave-nav::baking_scaling/5000",
            "unit": "ns",
            "value": 92607041.31
          },
          {
            "name": "astraweave-nav::pathfind_long_50_100_hops",
            "unit": "ns",
            "value": 14884.073308926549
          },
          {
            "name": "astraweave-nav::pathfind_medium_10_20_hops",
            "unit": "ns",
            "value": 48874.0567615427
          },
          {
            "name": "astraweave-nav::pathfind_short_2_5_hops",
            "unit": "ns",
            "value": 1940.7471009470848
          },
          {
            "name": "astraweave-nav::pathfinding_scaling/10",
            "unit": "ns",
            "value": 31469.254125274736
          },
          {
            "name": "astraweave-nav::pathfinding_scaling/100",
            "unit": "ns",
            "value": 4450717.170000002
          },
          {
            "name": "astraweave-nav::pathfinding_scaling/20",
            "unit": "ns",
            "value": 134373.25925866998
          },
          {
            "name": "astraweave-nav::pathfinding_scaling/50",
            "unit": "ns",
            "value": 940195.6722466887
          },
          {
            "name": "astraweave-nav::throughput_100_triangles/queries_per_second",
            "unit": "ns",
            "value": 6292.80111869286
          },
          {
            "name": "astraweave-nav::throughput_10k_triangles/queries_per_second",
            "unit": "ns",
            "value": 654693.8174255433
          },
          {
            "name": "astraweave-nav::throughput_1k_triangles/queries_per_second",
            "unit": "ns",
            "value": 63813.0781664491
          },
          {
            "name": "astraweave-nav::tile_commits_per_tick",
            "unit": "ns",
            "value": 7687.292673136977
          }
        ]
      }
    ]
  }
}