FINDORA_STORE_FILE=${FINDORA_HOME:-${HOME}/.findora}/cli2_data.sqlite
export FINDORA_SUBMIT_URL=${FINDORA_SUBMIT_URL:-https://testnet.findora.org:8669}
export FINDORA_ACCESS_URL=${FINDORA_ACCESS_URL:-https://testnet.findora.org:8668}

setup()
{
    # Start from a fresh state
    echo "Deleting $FINDORA_STORE_FILE..."
    rm -f $FINDORA_STORE_FILE || true
    bash -c '{ echo "$FINDORA_SUBMIT_URL"; echo "$FINDORA_ACCESS_URL"; } | $CLI2 setup'
}

debug_array()
{

    echo "Debugging array..."
    arr=("$@")
    COUNTER=0
    for i in "${arr[@]}"; do
        echo "[$COUNTER]$i"
        COUNTER=$((COUNTER + 1))
    done
}

debug_lines()
{
    debug_array "${lines[@]}"
}

check_line()
{
    line_number="$1"
    command_str="$2"
    command_str_length=$(expr length "$command_str")
    [ "${lines[$line_number]:0:$command_str_length}" = "$command_str" ]
}

# Similar to check_line above but verifies that either line "i" or line "i+1" has the expected value
# The reason for that is that sometimes the http req fails the first time and thus an error message is written at line i
check_line_err()
{
    line_number="$1"
    next_line_number=$line_number+1
    command_str="$2"
    command_str_length=$(expr length "$command_str")
    [[ ("${lines[$line_number]:0:$command_str_length}" == "$command_str") || ("${lines[$next_line_number]:0:$command_str_length}" == "$command_str") ]]
}

random_string()
{
    cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w ${1:-32} | head -n 1
}

PASSWORD="password"
PASSWORD_PROMPT="echo -e '$PASSWORD\n$PASSWORD\n'"
PASSWORD_PROMPT_YES="echo -e 'Y\n$PASSWORD\n$PASSWORD\n'"
SIMPLE_CONFIRM_WITH_PROMPT="echo -e '$PASSWORD\ny'"
SIMPLE_PASSWORD_PROMPT="echo -e '$PASSWORD'"
DOUBLE_CONFIRM_WITH_PROMPT="echo -e 'Y\nY\n'"
MEMO_ALICE_WITH_PROMPT="echo -e '$PASSWORD\nmemo_alice\n'"
ALICE_WITH_SEVERAL_PROMPTS="echo -e '$PASSWORD\n$PASSWORD\nY\nY\n'"
MEMO_ALICE_WITH_SEVERAL_PROMPTS="echo -e '$PASSWORD\nmemo_alice\n$PASSWORD\nY\nY\n'"
DEFINE_AND_ISSUE_ASSET_TYPE_WITH_BUILD="{ $PASSWORD_PROMPT |$CLI2 key-gen alice; } && \
                                     { echo y | $CLI2 query-ledger-state; } && \
                                     { $CLI2 initialize-transaction 0; } && \
                                     { $MEMO_ALICE_WITH_PROMPT | $CLI2 define-asset 0 alice TheBestAliceCoinsOnEarthV2; } && \
                                     { $PASSWORD_PROMPT | $CLI2 issue-asset 0 TheBestAliceCoinsOnEarthV2 0 10000; } && \
                                     { $PASSWORD_PROMPT | $CLI2 build-transaction; };"

DEFINE_ASSET_TYPE_WITH_SUBMIT_COMMANDS="{ $PASSWORD_PROMPT | $CLI2 key-gen alice; } && \
                              { echo y | $CLI2 query-ledger-state; } && \
                              { $CLI2 initialize-transaction 0; } && \
                              { $MEMO_ALICE_WITH_PROMPT | $CLI2 define-asset 0 alice AliceCoin; } && \
                              { $PASSWORD_PROMPT | $CLI2 build-transaction; } && \
                              { $DOUBLE_CONFIRM_WITH_PROMPT | $CLI2 submit 0; };"

# Enables to create a transfer for some amount from Alice to Bob
# Note that the change of the transfer is returned to Alice
get_transfer_prompt_transfer_asset()
{
    amount=$1
    change_amount=$2
    utxo_name=$3
    amount_confidential=$4 # Must be 'Y' or 'n'
    asset_confidential=$5 # Must be 'Y' or 'n'
    sender=$6
    receiver=$7
    receiver_local=$8 # Must be 'Y' or 'n'
    unlock=$9 # Must be 'Y' or 'n'

    if [[ $unlock == "Y" ]]
    then
        unlock_prompt_message="Y\n$PASSWORD"
    else
        unlock_prompt_message="n"
    fi

    COMMON_PROMPT_STR="echo -e '$utxo_name\n$amount\n$amount_confidential\n$asset_confidential\n$receiver\nY\n$change_amount\n$amount_confidential\n$asset_confidential\n$sender\n$unlock_prompt_message\n"
    if [[ "$receiver_local" == "true" ]]; then
        PROMPT_TRANSFER_ASSET="${COMMON_PROMPT_STR}$unlock_prompt_message\n$PASSWORD\nY\nY\n'"
    else
        PROMPT_TRANSFER_ASSET="${COMMON_PROMPT_STR}$PASSWORD\nY\nY\n'"
    fi
    echo $PROMPT_TRANSFER_ASSET
}

get_utxo_name()
{
    utxo_index="0"
    utxo_name=${lines[$utxo_index]:5:-1}
    echo $utxo_name
}

transfer_assets()
{
    amount=$1
    change_amount=$2
    amount_confidential=$3
    asset_confidential=$4
    filter_by_sender=$5
    sender=$6
    receiver=$7
    receiver_local=$8
    unlock=$9

    tx_name=$(random_string 16)
    echo "TX_NAME: $tx_name"

    # If the filter_by_sender is not provided list all the unspent txos
    if [[ $filter_by_sender == "true" ]]; then
        run bash -c "$CLI2 list-txos --unspent=true --id=$sender"
    else
        run bash -c "$CLI2 list-txos --unspent=true"
    fi
    [ "$status" -eq 0 ]

    utxo_name=$(get_utxo_name)

    echo "UTXO_NAME=$utxo_name"
    [ "$status" -eq 0 ]

    run bash -c "$CLI2 initialize-transaction $tx_name"

    PROMPT=$(get_transfer_prompt_transfer_asset \
                "$amount" \
                "$change_amount" \
                "$utxo_name" \
                "$amount_confidential" \
                "$asset_confidential" \
                "$sender" \
                "$receiver" \
                "$receiver_local" \
                "$unlock")

    run bash -c "$PROMPT | $CLI2 transfer-assets --builder=$tx_name"
    [ "$status" -eq 0 ]

    run bash -c "$PASSWORD_PROMPT | $CLI2 build-transaction"
    [ "$status" -eq 0 ]

    TX_ID="${lines[0]:10:-1}"
    echo $"Transaction ID: $TX_ID"

    run bash -c "$DOUBLE_CONFIRM_WITH_PROMPT | $CLI2 submit $TX_ID;"
    [ "$status" -eq 0 ]
}